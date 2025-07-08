use pyo3_build_config::{InterpreterConfig, PythonVersion};
use std::env;
use std::process::Command;

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
    println!("cargo:rerun-if-changed=build.rs");
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

    let python_executable = env::var("PYTHON")
        .or_else(|_| env::var("PYO3_PYTHON"))
        .unwrap_or_else(|_| "python3".to_string());

    println!("cargo:warning=Using Python executable: {python_executable}");

    // Detect Python version for more accurate linking
    let version = detect_python_version(&python_executable);
    println!("cargo:warning=Detected Python version: {version}");

    // Try pkg-config first for the most accurate linking configuration
    if try_pkg_config_linking() {
        println!("cargo:warning=Successfully configured Python linking via pkg-config");
        return;
    }

    // Fallback to manual library detection and linking
    add_python_library_paths(&python_executable);
    link_python_library(&version);
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

fn detect_python_version(python_executable: &str) -> String {
    if let Ok(output) = Command::new(python_executable)
        .args([
            "-c",
            "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')",
        ])
        .output()
    {
        if output.status.success() {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
    }
    "3.12".to_string() // Default fallback
}

fn try_pkg_config_linking() -> bool {
    // Try python3-config first (more reliable for Python linking)
    if try_python_config_linking() {
        return true;
    }

    // Fallback to pkg-config
    if let Ok(output) = Command::new("pkg-config")
        .args(["--libs", "python3"])
        .output()
    {
        if output.status.success() {
            let libs = String::from_utf8_lossy(&output.stdout);
            if !libs.trim().is_empty() {
                // Parse and add the library flags
                for flag in libs.split_whitespace() {
                    if let Some(path) = flag.strip_prefix("-L") {
                        println!("cargo:rustc-link-search=native={path}");
                    } else if let Some(lib) = flag.strip_prefix("-l") {
                        println!("cargo:rustc-link-lib={lib}");
                    }
                }
                return true;
            }
        }
    }
    false
}

fn try_python_config_linking() -> bool {
    let mut success = false;

    // Get library search paths from python3-config --ldflags
    if let Ok(output) = Command::new("python3-config").args(["--ldflags"]).output() {
        if output.status.success() {
            let ldflags = String::from_utf8_lossy(&output.stdout);
            if !ldflags.trim().is_empty() {
                parse_and_add_link_flags(&ldflags);
                success = true;
            }
        }
    }

    // Get libraries from python3-config --libs --embed (for embedding Python >= 3.8)
    if let Ok(output) = Command::new("python3-config")
        .args(["--libs", "--embed"])
        .output()
    {
        if output.status.success() {
            let libs = String::from_utf8_lossy(&output.stdout);
            if !libs.trim().is_empty() {
                parse_and_add_link_flags(&libs);
                success = true;
            }
        }
    } else {
        // Fallback to python3-config --libs if --embed is not supported
        if let Ok(output) = Command::new("python3-config").args(["--libs"]).output() {
            if output.status.success() {
                let libs = String::from_utf8_lossy(&output.stdout);
                if !libs.trim().is_empty() {
                    parse_and_add_link_flags(&libs);
                    success = true;
                }
            }
        }
    }

    success
}

fn parse_and_add_link_flags(libs: &str) {
    // Parse and add the library flags
    for flag in libs.split_whitespace() {
        if let Some(path) = flag.strip_prefix("-L") {
            println!("cargo:rustc-link-search=native={path}");
        } else if let Some(lib) = flag.strip_prefix("-l") {
            println!("cargo:rustc-link-lib={lib}");
        }
    }
}

fn add_python_library_paths(python_executable: &str) {
    // Add common Python library search paths
    let search_paths = ["/usr/local/lib", "/usr/lib", "/usr/lib/x86_64-linux-gnu"];

    for path in &search_paths {
        println!("cargo:rustc-link-search=native={path}");
    }

    // Get Python prefix and add its lib directory
    if let Ok(output) = Command::new(python_executable)
        .args(["-c", "import sys; print(sys.prefix)"])
        .output()
    {
        if output.status.success() {
            let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("cargo:rustc-link-search=native={prefix}/lib");

            // Add version-specific config directories
            if let Ok(version_output) = Command::new(python_executable)
                .args([
                    "-c",
                    "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')",
                ])
                .output()
            {
                if version_output.status.success() {
                    let version = String::from_utf8_lossy(&version_output.stdout)
                        .trim()
                        .to_string();
                    println!("cargo:rustc-link-search=native={prefix}/lib/python{version}/config-{version}-x86_64-linux-gnu");
                }
            }
        }
    }
}

fn link_python_library(version: &str) {
    // Try to link with the most specific Python library available
    let lib_names = [format!("python{version}"), "python3".to_string()];

    for lib_name in &lib_names {
        if try_link_library(lib_name) {
            println!("cargo:warning=Successfully linked with {lib_name}");
            return;
        }
    }

    // Fallback: try dynamic lookup for PyO3 extension modules
    println!("cargo:warning=Using dynamic lookup for Python symbols");
    println!("cargo:rustc-link-arg=-undefined");
    println!("cargo:rustc-link-arg=dynamic_lookup");
}

fn try_link_library(lib_name: &str) -> bool {
    // Check if the library exists in the search paths
    let search_paths = ["/usr/local/lib", "/usr/lib", "/usr/lib/x86_64-linux-gnu"];

    for path in search_paths {
        let lib_path = format!("{path}/lib{lib_name}.so");
        if std::path::Path::new(&lib_path).exists() {
            println!("cargo:rustc-link-lib={lib_name}");
            return true;
        }
    }
    false
}
