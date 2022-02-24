use std::io::Result;

fn main() -> Result<()> {
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile(&["src/spectate.proto"], &["src/"])?;
    //prost_build::compile_protos(&["src/spectate.proto"], &["src/"])?;
    Ok(())
}
