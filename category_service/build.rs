fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .build_server(false)   // لا نقدّم gRPC، فقط نستهلكه
        .build_client(true)
        .compile_protos(&["proto/inventory.proto"], &["proto"])?;
    Ok(())
}