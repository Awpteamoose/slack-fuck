use once_cell::sync::Lazy;
use windows::{core::s, Win32::{Foundation::{BOOL, HMODULE, TRUE}, System::{Diagnostics::Debug::ReadProcessMemory, SystemServices::DLL_PROCESS_ATTACH, Threading::GetCurrentProcess}, UI::Shell::{Shell_NotifyIconA, NOTIFYICONDATAA, NOTIFYICONDATAW, NOTIFY_ICON_MESSAGE}}};

// static ORIGINAL_SHELL_NOTIFY_ICON: Lazy<unsafe fn(dw_message: NOTIFY_ICON_MESSAGE, lp_data: *const NOTIFYICONDATAA) -> BOOL> = Lazy::new(|| Shell_NotifyIconA);
static ORIGINAL_SHELL_NOTIFY_ICON_A: Lazy<unsafe extern "system" fn(dw_message: NOTIFY_ICON_MESSAGE, lp_data: *const NOTIFYICONDATAA) -> BOOL> = Lazy::new(|| {
	let h_shell32 = unsafe { windows::Win32::System::LibraryLoader::GetModuleHandleA(s!("shell32.dll")) }.unwrap();
	let p_func = unsafe { windows::Win32::System::LibraryLoader::GetProcAddress(h_shell32, s!("Shell_NotifyIconA")) }.unwrap();
	unsafe { std::mem::transmute(p_func) }
});
static ORIGINAL_SHELL_NOTIFY_ICON_W: Lazy<unsafe extern "system" fn(dw_message: NOTIFY_ICON_MESSAGE, lp_data: *const NOTIFYICONDATAW) -> BOOL> = Lazy::new(|| {
	let h_shell32 = unsafe { windows::Win32::System::LibraryLoader::GetModuleHandleA(s!("shell32.dll")) }.unwrap();
	let p_func = unsafe { windows::Win32::System::LibraryLoader::GetProcAddress(h_shell32, s!("Shell_NotifyIconW")) }.unwrap();
	unsafe { std::mem::transmute(p_func) }
});

unsafe extern "system" fn hooked_shell_notify_icon_a(dw_message: NOTIFY_ICON_MESSAGE, lp_data: *const NOTIFYICONDATAA) -> BOOL {
	// TODO: custom logic

	(ORIGINAL_SHELL_NOTIFY_ICON_A)(dw_message, lp_data)
}
unsafe extern "system" fn hooked_shell_notify_icon_w(dw_message: NOTIFY_ICON_MESSAGE, lp_data: *const NOTIFYICONDATAW) -> BOOL {
	// TODO: custom logic

	(ORIGINAL_SHELL_NOTIFY_ICON_W)(dw_message, lp_data)
}

#[no_mangle]
pub extern "system" fn DllMain(h_module: HMODULE, ul_reason_for_call: u32, lp_reserved: *mut std::ffi::c_void) -> BOOL {
	if ul_reason_for_call == DLL_PROCESS_ATTACH {
		Lazy::force(&ORIGINAL_SHELL_NOTIFY_ICON_A);
		Lazy::force(&ORIGINAL_SHELL_NOTIFY_ICON_W);

		// patch Shell_NotifyIconA
		unsafe {
			let h_shell32 = windows::Win32::System::LibraryLoader::GetModuleHandleA(s!("shell32.dll")).unwrap();
			let p_func = windows::Win32::System::LibraryLoader::GetProcAddress(h_shell32, s!("Shell_NotifyIconA")).unwrap();

			let mut original_bytes = [0u8; 6];
			ReadProcessMemory(GetCurrentProcess(), p_func as _, original_bytes.as_mut_ptr().cast(), 6, None).unwrap();

			let mut patch = [0; 6];
			patch[0] = 0x68;
			patch[1..5].copy_from_slice(&(hooked_shell_notify_icon_a as usize).to_ne_bytes()[..4]);
			patch[5] = 0xC3;
		}
	}

	true.into()
}
