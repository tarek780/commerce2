fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                "proto/order.proto",
                "proto/inventory.proto",
                "proto/payment.proto",
            ],
            &["proto"],
        )?;
    Ok(())
}