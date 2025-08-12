# Build script for PECOS.jl
# This is run automatically when the package is installed

# For development: build the Rust library
if get(ENV, "PECOS_DEV", "false") == "true"
    println("Building PECOS FFI library for development...")
    cd(joinpath(@__DIR__, "..", "..", "pecos-julia-ffi")) do
        run(`cargo build --release`)
    end
else
    # For production: would download pre-built binaries or use JLL
    @info "Using system PECOS library or JLL"
end
