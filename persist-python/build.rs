use pyo3_build_config::{InterpreterConfig, PythonVersion};
use std::env;

fn main() {
    // Check if we're running under tarpaulin for special handling
    let is_tarpaulin = env::var("CARGO_TARPAULIN").is_ok()
        || env::var("TARPAULIN").is_ok()
        || env::args().any(|arg| arg.contains("tarpaulin"));

    if is_tarpaulin {
        println!(
            "cargo:warning=Tarpaulin detected - using compatible Python linking configuration"
        );
        configure_for_tarpaulin();
        return;
    }

    // Configure PyO3 build - use from_interpreter with current python
    let python_interpreter = env::var("PYTHON").unwrap_or_else(|_| "python3".to_string());

    let config = match InterpreterConfig::from_interpreter(python_interpreter) {
        Ok(config) => config,
        Err(_) => {
            // Fallback to trying to find a Python interpreter
            println!("cargo:warning=Failed to get Python interpreter config, using defaults");
            return;
        }
    };

    // Ensure we're using a supported Python version
    let min_version = PythonVersion { major: 3, minor: 8 };
    if config.version < min_version {
        panic!(
            "Python 3.8 or later is required, found {:?}",
            config.version
        );
    }

    // Configure linking based on Python version
    configure_python_linking(&config);

    // Set environment variables for consistent behavior
    println!("cargo:rerun-if-env-changed=PYTHONPATH");
    println!("cargo:rerun-if-env-changed=PYTHON_SYS_EXECUTABLE");
    println!("cargo:rerun-if-env-changed=PYTHON");
    println!("cargo:rerun-if-env-changed=CARGO_TARPAULIN");
    println!("cargo:rerun-if-env-changed=TARPAULIN");
}

fn configure_for_tarpaulin() {
    // For tarpaulin, use a more conservative approach that avoids problematic linking
    // Use ABI3 stable interface which is more compatible with different Python versions
    println!("cargo:rustc-cfg=Py_LIMITED_API");

    // Try to detect Python installation
    if let Ok(python_executable) = env::var("PYTHON").or_else(|_| env::var("PYO3_PYTHON")) {
        println!("cargo:warning=Using Python executable: {python_executable}");
    }

    // For tarpaulin, we'll rely on ABI3 and avoid version-specific linking
    // This prevents the PyObject_CallMethodObjArgs linking issues
    println!("cargo:rustc-link-lib=python3");

    // Add common Python library search paths
    if let Ok(python_path) = std::process::Command::new("python3")
        .args(["-c", "import sys; print(sys.prefix)"])
        .output()
    {
        if python_path.status.success() {
            let prefix_cow = String::from_utf8_lossy(&python_path.stdout);
            let prefix = prefix_cow.trim();
            println!("cargo:rustc-link-search=native={prefix}/lib");
            println!("cargo:rustc-link-search=native={prefix}/lib/python3.12/config-3.12-x86_64-linux-gnu");
            println!("cargo:rustc-link-search=native={prefix}/lib/python3.11/config-3.11-x86_64-linux-gnu");
            println!("cargo:rustc-link-search=native={prefix}/lib/python3.10/config-3.10-x86_64-linux-gnu");
        }
    }
}

fn configure_python_linking(config: &InterpreterConfig) {
    // For Python 3.13 compatibility, we need to ensure proper linking
    let python_313 = PythonVersion {
        major: 3,
        minor: 13,
    };

    if config.version >= python_313 {
        // Python 3.13+ requires more careful linking
        println!("cargo:warning=Configuring for Python 3.13+ compatibility");

        // Ensure we have the correct Python library paths
        if let Some(lib_dir) = &config.lib_dir {
            println!("cargo:rustc-link-search=native={lib_dir}");
        }

        // Link against the specific Python library
        if let Some(lib_name) = &config.lib_name {
            println!("cargo:rustc-link-lib=dylib={lib_name}");
        }

        // Additional linking flags for Python 3.13 compatibility
        println!("cargo:rustc-link-arg=-Wl,--no-as-needed");
        println!("cargo:rustc-link-arg=-Wl,--allow-undefined-symbols");
    } else {
        // For older Python versions, use standard linking
        if let Some(lib_dir) = &config.lib_dir {
            println!("cargo:rustc-link-search=native={lib_dir}");
        }

        if let Some(lib_name) = &config.lib_name {
            println!("cargo:rustc-link-lib=dylib={lib_name}");
        }
    }
}
