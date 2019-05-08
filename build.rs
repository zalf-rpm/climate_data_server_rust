extern crate capnpc;

fn main() {
    ::capnpc::CompilerCommand::new().file("capnproto_schemas/climate_data.capnp").run().unwrap();
    ::capnpc::CompilerCommand::new().file("capnproto_schemas/date.capnp").run().unwrap();
    ::capnpc::CompilerCommand::new().file("capnproto_schemas/geo_coord.capnp").run().unwrap();
}
