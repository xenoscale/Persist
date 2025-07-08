use pyo3_build_config::{PythonVersion, InterpreterConfig};

fn main() {
    // Configure PyO3 build - use from_interpreter with current python
    let python_interpreter = std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string());
    
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
        panic!("Python 3.8 or later is required, found {:?}", config.version);
    }
    
    // For Python 3.13 compatibility, we need to ensure proper linking
    let python_313 = PythonVersion { major: 3, minor: 13 };
    if config.version >= python_313 {
        // Ensure we have the correct Python library paths
        if let Some(lib_dir) = &config.lib_dir {
            println!("cargo:rustc-link-search=native={}", lib_dir);
        }
        
        // Link against the specific Python library
        if let Some(lib_name) = &config.lib_name {
            println!("cargo:rustc-link-lib=dylib={}", lib_name);
        }
    }
    
    // Set environment variables for consistent behavior
    println!("cargo:rerun-if-env-changed=PYTHONPATH");
    println!("cargo:rerun-if-env-changed=PYTHON_SYS_EXECUTABLE");
    println!("cargo:rerun-if-env-changed=PYTHON");
}
