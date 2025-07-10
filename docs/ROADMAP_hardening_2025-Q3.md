# Persist – Q3 2025 Hardening Road‑map  
_"From MVP to Production‑grade"_

This guide is split into **11 tracks**.  
Work through them **in order** – several later tasks assume earlier refactors are finished.

| Track | Outcome | Est. Effort |
|-------|---------|-------------|
| 1. Workspace & Feature flags | Slim build, optional async | ½ day |
| 2. Async Core API | No nested runtimes | 1 day |
| 3. Common Retry/Back‑off crate | Single impl for all back‑ends | ½ day |
| 4. Local adapter (atomic & safe) | Crash‑safe writes | 1 day |
| 5. S3 adapter | Streaming/multipart & metrics | 1 day |
| 6. GCS adapter | Resumable upload & metrics | 1 day |
| 7. Observability unification | Same spans + Prom metrics | ½ day |
| 8. Security hardening | Path traversal, log redaction | ½ day |
| 9. Comprehensive test‑suite | Fake‑GCS, LocalStack, crash sim | 1 day |
| 10. CI matrix update | Linux + macOS + Windows | ½ day |
| 11. Docs & DX polish | Builder, CHANGELOG, README | ½ day |

---

## 1 Workspace & Feature‑flags

1. **Gate Tokio behind `async-rt`** in `persist‑core/Cargo.toml`:

   ```toml
   [features]
   default = ["local"]
   s3      = ["aws-config", "aws-sdk-s3", "aws-smithy-runtime-api"]
   gcs     = ["google-cloud-storage", "google-cloud-auth"]
   async-rt = ["dep:tokio"]          # NEW
   ```

   Move `tokio = { version = "1.28", features = ["full"], optional = true }` under `[dependencies]`.

2. **Shared dependency versions** – create a `[workspace.dependencies]` table in the root `Cargo.toml` and delete duplicates from sub‑crates.

3. **Patch‑pin all network crates**:
   - `aws-sdk-s3      = { version = "1.96.*", optional = true }`
   - `google-cloud-storage = { version = "0.24.*", optional = true }`
   - `tracing         = "0.1.*"`

## 2 Make the storage layer async‑native

### 2.1 Define the trait

```rust
#[async_trait::async_trait]
pub trait StorageAdapter {
    async fn save(&self, reader: impl AsyncRead + Send + 'static, path: &str) -> Result<()>;
    async fn load(&self, path: &str) -> Result<impl AsyncRead>;
    async fn exists(&self, path: &str) -> Result<bool>;
    async fn delete(&self, path: &str) -> Result<()>;
}
```

* Keep the current sync wrapper for callers that cannot be async:

```rust
pub struct BlockingStorage<A: StorageAdapter>(A);

impl<A: StorageAdapter> BlockingStorage<A> {
    pub fn save(&self, bytes: &[u8], path: &str) -> Result<()> {
        GLOBAL_RT.block_on(self.0.save(Bytes::copy_from_slice(bytes), path))
    }
}
```

### 2.2 Global runtime

```rust
static GLOBAL_RT: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_cpus::get().max(4))
        .enable_all()
        .build()
        .unwrap()
});
```

Adapters now call `GLOBAL_RT.handle().clone()` when they need a Handle.

## 3 Shared retry & back‑off

Add crate `persist-retry`:

```rust
pub async fn with_backoff<F, T>(op_name: &'static str, mut f: F) -> Result<T>
where
    F: FnMut(usize) -> BoxFuture<'_, Result<T>>,
{
    let policy = backoff::ExponentialBackoffBuilder::new()
        .with_max_elapsed_time(Some(Duration::from_secs(30)))
        .build();
    retry(policy, || async {
        f(backoff::current_attempt()).await
    })
    .await
    .map_err(|e| PersistError::transient(op_name, e))
}
```

Both cloud adapters replace manual loops + `thread::sleep`.

## 4 Local adapter hardening

### 4.1 Atomic, durable write

```rust
fn atomic_write(full: &Path, data: &[u8]) -> Result<()> {
    let dir = full.parent().ok_or_else(|| PersistError::storage("no parent"))?;
    let tmp = NamedTempFile::new_in(dir)?;
    tmp.as_file().write_all(data)?;
    tmp.as_file().sync_all()?;          // flush file
    tmp.persist_noclobber(full)?;       // rename is atomic
    File::open(dir)?.sync_all()?;       // flush directory entry
    Ok(())
}
```

### 4.2 Escape & symlink guard

```rust
let canon = full_path.canonicalize()?;
if let Some(base) = &self.base_dir {
    if !canon.starts_with(base) {
        return Err(PersistError::validation("path escapes base_dir"));
    }
}
```

Add `OpenOptionsExt::custom_flags(libc::O_NOFOLLOW)` on Unix for save.

### 4.3 Observability

Add tracing spans and:
- `persist_storage_ops_total{backend="local",op="save"}`
- `persist_storage_latency_seconds_bucket{backend="local",op="load"}`

## 5 S3 adapter

1. **Streaming uploads**
   ```rust
   let stream = ByteStream::read_from(reader, data_len).await?;
   client.put_object().bucket(bucket).key(key).body(stream).send().await?;
   ```
   For `data_len ≥ 8_388_608` start multipart upload with `UploadPart`.

2. **Reuse buffer across retries**: accept `Bytes` not `&[u8]`.

3. **Exists()**
   ```rust
   match client.head_object().bucket(...).key(...).send().await {
       Ok(_) => Ok(true),
       Err(SdkError::ServiceError{err, ..}) if err.code() == Some("404") => Ok(false),
       Err(e) => Err(e.into()),
   }
   ```

4. **Server‑side encryption flags**
   ```rust
   if let Some(kms) = config.kms_key_id {
       req = req.server_side_encryption("aws:kms")
                .ssekms_key_id(kms);
   }
   ```

5. **Metrics**
   ```rust
   histogram!("persist_transfer_bytes", data_len as f64, "backend"=>"s3","op"=>"put");
   ```

## 6 GCS adapter

1. **Resumable upload**
   ```rust
   let sess = client.object().start_resumable_upload(req, "application/gzip").await?;
   tokio::io::copy(&mut reader, &mut sess).await?;
   sess.finish().await?;
   ```

2. Fill `name` in `UploadObjectRequest` to avoid deprecation warnings.

3. **CRC32C validation**
   ```rust
   let (meta, reader) = client.object().download(&req).await?;
   let expected = meta.crc32c();
   let bytes     = read_to_end(reader).await?;
   verify_crc32c(&bytes, expected)?;
   ```

4. **Credential injection** Accept a `service_account_json: Option<String>` in the builder and pass to `ClientConfig::with_service_account_key`.

## 7 Observability unification

Create `observability::record_op(backend, op, size, latency, retries);` call from every adapter.

Prometheus names:
- `persist_storage_ops_total{backend,op}`
- `persist_storage_bytes_total{backend,op}`
- `persist_storage_latency_seconds_bucket{backend,op}`
- `persist_storage_retries_total{backend,op}`

Update Grafana dashboards accordingly.

## 8 Security hardening

1. **Redact secrets**
   ```rust
   tracing_subscriber::fmt()
       .with_filter(tracing_subscriber::EnvFilter::from_default_env())
       .with_redaction(Regex::new(r"(AWS|GOOGLE)_.*").unwrap());
   ```

2. Do not log full object paths at INFO; include only bucket + basename unless `debug!`.

3. **IAM least privilege** – add policy snippets to `docs/security.md`.

## 9 Test‑suite expansion

| Scenario | Tool | Notes |
|----------|------|-------|
| GCS emulator | fake-gcs-server | Launch in GitHub Actions service container. |
| S3 LocalStack | Already present; add Windows job (use act locally). | |
| Crash‑consistency | Fork a child, write snapshot, kill -9; parent verifies no partial file. | |
| Path‑traversal guard | Try "../../evil" & expect Err. | |
| Doctests | Mark all rustdoc examples with ````rust,ignore` then enable `--test`. | |

## 10 CI matrix

```yaml
strategy:
  matrix:
    os: [ubuntu-latest, macos-latest, windows-latest]
    features: ["local", "local,s3", "local,gcs", "local,s3,gcs,metrics,async-rt"]
```

Add coverage (grcov) and clippy with `--all-features`.

## 11 Docs & Developer Experience

1. **Builder pattern**
   ```rust
   let s3 = S3StorageAdapter::builder()
               .bucket("snapshots")
               .endpoint("http://localhost:4566")
               .kms_key_id("alias/persist")
               .build()?;
   ```

2. Update README feature matrix; note that `async‑rt` is now opt‑in.

3. Add CHANGELOG entries for each track; bump version to `0.2.0-alpha`.

## Done!

When all tracks are complete:
```bash
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo publish --dry-run
```

You now have a production‑ready snapshot engine with pluggable, durable and observable storage adapters. 🎉

---

### Key file references

* Unconditional Tokio dep  
* Default features include S3 + GCS  
* Local adapter uses direct `fs::write`  
* Sync trait definition
