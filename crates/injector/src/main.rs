use dll_syringe::{process::{OwnedProcess, Process}, Syringe};

fn main() {
	let mut payloads = Vec::new();
	for process in OwnedProcess::find_all_by_name("slack.exe") {
		let pid = process.pid().unwrap();
		let is_x64 = process.is_x64().unwrap();
		let syringe: &'static _ = &*Box::leak(Box::new(Syringe::for_process(process)));
		let injected_payload = syringe.inject("target/release/dll.dll");
		println!("process id {pid}, x64? {is_x64}, injected? {}", injected_payload.is_ok());
		if let Ok(x) = injected_payload {
			payloads.push((syringe, x));
		}
	}

	std::io::stdin().read_line(&mut String::new()).unwrap();

	for (syringe, payload) in payloads {
		syringe.eject(payload).unwrap();
	}
}
