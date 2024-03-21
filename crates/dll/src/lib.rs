use windows::{core::{s, PCWSTR}, Win32::{Foundation::{BOOL, HMODULE}, Graphics::Gdi::{CreateCompatibleDC, DeleteDC, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS}, System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH}, UI::{Shell::{NIF_ICON, NIF_TIP, NIM_MODIFY, NOTIFYICONDATAW, NOTIFY_ICON_MESSAGE}, WindowsAndMessaging::{GetIconInfoExW, ICONINFOEXW}}}};
use std::io::Write;

// it appears that the easiest way is by inspecting NIF_TIP
// there's 3 kinds of tip:
// * "No unread messages" - no notifs (regular icon)
// * "You have unread messages" - channel messages (blue blob icon)
// * "You have X notification(s)" - dms/mentions/etc (red blob icon)
//
// last Shell_NotifyIconW call is a reliable source of info, however it is only called when there's a change,
// even if the channel or dm is in focus
// so to avoid spurious activations - might need to stagger activations a little bit
// to show last message would require hitting the api - https://api.slack.com/messaging/retrieving
// "x notifications" means we need to fetch dms, filter by unread count > 0, etc
// "unread messages" means we need to fetch channels, filter by unread count > 0, etc
// there is a slack api method to mark shit as read
//
// just use token xoxc-157807579798-158991353072-6824754999410-22a90d0d561d1e58a1e2fbd979dd6290c5a266ee41cb6bb969553f7f98bc57a4
// taken from /slackdevtools
//
// GET https://slack.com/api/conversations.list?exclude_archived=true&types=public_channel,private_channel,mpim,im&limit=1000&token=xoxc-XXX
//     * use cursor query param and cursor from reply to get all
// GET https://slack.com/api/conversations.info?channel=XXX&token=xoxc-XXX
//     * has `res.channel.last_read` which is a ts of last read, can be then compared with ts of last message
// GET https://slack.com/api/conversations.history?channel=C05SHCK1939&limit=1&token=xoxc-
//     * gets newest message first
// POST https://slack.com/api/conversations.mark
//     -H Content-Type: application/x-www-form-urlencoded
//     Form: channel=C1341, ts=123617.231q, token=xoxc-XXX
//     * will mark message with ts (and all preceeding) as read

retour::static_detour! {
	static Shell_NotifyIconW: unsafe extern "system" fn(NOTIFY_ICON_MESSAGE, *const NOTIFYICONDATAW) -> BOOL;
}

fn log_file() -> std::fs::File {
	std::fs::OpenOptions::new().append(true).create(true).open("D:\\projects\\arduino\\slack-fuck\\log.txt").unwrap()
}

fn hooked_shell_notify_icon_w(dw_message: NOTIFY_ICON_MESSAGE, lp_data: *const NOTIFYICONDATAW) -> BOOL {
	unsafe {
		if dw_message != NIM_MODIFY { return Shell_NotifyIconW.call(dw_message, lp_data); }

		let mut file = log_file();

		let flags = (*lp_data).uFlags;

		if flags.contains(NIF_ICON) {
			let icon = (*lp_data).hIcon;
			let icon_info = {
				let mut x = ICONINFOEXW {
					cbSize: std::mem::size_of::<ICONINFOEXW>() as _,
					..Default::default()
				};
				GetIconInfoExW(icon, &mut x).unwrap();
				x
			};
			let bmp = {
				let mut x = BITMAP::default();
				assert!(GetObjectW(icon_info.hbmColor, std::mem::size_of::<BITMAP>() as _, Some((&mut x as *mut BITMAP).cast())) > 0);
				x
			};
			let mut bmi = BITMAPINFO {
				bmiHeader: BITMAPINFOHEADER {
					biSize: std::mem::size_of::<BITMAPINFOHEADER>() as _,
					biWidth: bmp.bmWidth,
					biHeight: -bmp.bmHeight,
					biPlanes: bmp.bmPlanes,
					biBitCount: bmp.bmBitsPixel,
					biCompression: BI_RGB.0,
					biSizeImage: (bmp.bmWidth * bmp.bmHeight) as u32,
					..Default::default()
				},
				..Default::default()
			};

			// writeln!(&mut file, "icon_info: {icon_info:?}, bmp {bmp:?}, bmi: {bmi:?}").unwrap();

			let hdc_screen = GetDC(None);
			let hdc_mem = CreateCompatibleDC(hdc_screen);

			let mut dibits = vec![0u8; (bmp.bmWidth * bmp.bmHeight * bmp.bmBitsPixel as i32 / 8) as usize];

			GetDIBits(hdc_screen, icon_info.hbmColor, 0, bmp.bmHeight as _, Some(dibits.as_mut_ptr().cast()), &mut bmi, DIB_RGB_COLORS);
			// let image = image::RgbaImage::from_fn(bmp.bmWidth as u32, bmp.bmHeight as u32, |x, y| {
			//     let offset = ((x + y * bmp.bmWidth as u32) * 4) as usize;
			//     image::Rgba([dibits[offset + 2], dibits[offset + 1], dibits[offset], dibits[offset + 3]])
			// });

			DeleteDC(hdc_mem);
			ReleaseDC(None, hdc_screen);

			let filename = format!("D:\\projects\\arduino\\slack-fuck\\{}.png", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
			// image.save(&filename).unwrap();
			writeln!(&mut file, "icon_w called - NIF_ICON - image saved as: {filename}").unwrap();
		} else if flags.contains(NIF_TIP) {
			let tip = PCWSTR::from_raw((*lp_data).szTip.as_ptr()).to_string().unwrap();
			writeln!(&mut file, "icon_w called - NIF_TIP - tip: {tip}").unwrap();
		}

		Shell_NotifyIconW.call(dw_message, lp_data)
	}
}

#[no_mangle]
pub extern "system" fn DllMain(_: HMODULE, ul_reason_for_call: u32, _: *mut std::ffi::c_void) -> BOOL {
	match ul_reason_for_call {
		DLL_PROCESS_ATTACH => unsafe {
			let h_shell32 = windows::Win32::System::LibraryLoader::GetModuleHandleA(s!("shell32.dll")).unwrap();

			let p_func = windows::Win32::System::LibraryLoader::GetProcAddress(h_shell32, s!("Shell_NotifyIconW")).unwrap() as usize;
			Shell_NotifyIconW.initialize(std::mem::transmute(p_func), hooked_shell_notify_icon_w).unwrap()
				.enable().unwrap();

			writeln!(&mut log_file(), "injected").unwrap();
		},
		DLL_PROCESS_DETACH => unsafe {
			Shell_NotifyIconW.disable().unwrap();

			writeln!(&mut log_file(), "ejected").unwrap();
		},
		_ => {},
	}
	
	true.into()
}
