#[test]
#[ignore]
fn local_test() {
	env_logger::init();
	let cl = nefit_client::Client::new("", "", "");
	let cm = cl.connect().unwrap();
	//	cm.set_temp_room_manual(19.).unwrap();
	dbg!(cm.status().unwrap());
	dbg!(cm.outdoor_temp().unwrap());
	dbg!(cm.system_pressure().unwrap());
	dbg!(cm.supply_temp().unwrap());
}
