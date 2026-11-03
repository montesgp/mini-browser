// Derived from the cef-rs `bundle-cef-app` tool (https://github.com/tauri-apps/cef-rs),
// licensed Apache-2.0 OR MIT.

use std::{env, ffi::c_void, os::windows::ffi::OsStrExt, path::Path, path::PathBuf, ptr};

use anyhow::{Context, bail};
use cef::build_util::win::build_bundle;
use clap::Parser;

/// The app icon; see `assets/icon/icon.svg`.
const ICON: &[u8] = include_bytes!("../../assets/icon/icon.ico");

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// Package and executable name of the app.
    name: String,
    #[arg(long, default_value_t = false)]
    release: bool,
    /// Bundle directory; defaults to the current directory.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

pub fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let output = match args.output {
        Some(output) => output,
        None => env::current_dir()?,
    };

    let bundle_path = build_bundle(&output, &args.name, args.release)?;
    set_icon(&bundle_path)?;
    println!("Run the app from {}", bundle_path.display());
    Ok(())
}

/// Resource type ids from `winuser.h`.
const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
/// Name of the app's icon group. Named resources come before numbered ones, so Explorer and the
/// taskbar pick this group over the generic icon CEF's bootstrap carries (group 32512).
const ICON_GROUP_NAME: &str = "MINIBROWSER";
/// First resource id of the app's icon images, above the bootstrap's own (1 to 8).
const FIRST_ICON_ID: u16 = 101;
const LANG_NEUTRAL: u16 = 0;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn BeginUpdateResourceW(file_name: *const u16, delete_existing: i32) -> *mut c_void;
    fn UpdateResourceW(
        update: *mut c_void,
        kind: *const u16,
        name: *const u16,
        language: u16,
        data: *const c_void,
        size: u32,
    ) -> i32;
    fn EndUpdateResourceW(update: *mut c_void, discard: i32) -> i32;
}

/// Adds the app icon to the bundle's executable. With the sandbox the executable is CEF's
/// prebuilt `bootstrap.exe`, renamed, so the icon cannot come from the app's build; it is written
/// into the copied executable's resources instead.
fn set_icon(executable: &Path) -> anyhow::Result<()> {
    let (group, images) = icon_resources(ICON, FIRST_ICON_ID)?;
    let file: Vec<u16> = executable
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let group_name: Vec<u16> = ICON_GROUP_NAME.encode_utf16().chain(Some(0)).collect();
    // SAFETY: the strings are NUL-terminated and outlive the calls, the data pointers and sizes
    // come from live slices, and the update handle is used only between Begin and End.
    unsafe {
        let update = BeginUpdateResourceW(file.as_ptr(), 0);
        if update.is_null() {
            bail!("cannot open {} to set its icon", executable.display());
        }
        let mut ok = images.iter().all(|(id, data)| {
            UpdateResourceW(
                update,
                int_resource(RT_ICON),
                int_resource(*id),
                LANG_NEUTRAL,
                data.as_ptr().cast(),
                u32::try_from(data.len()).unwrap_or(u32::MAX),
            ) != 0
        });
        ok = ok
            && UpdateResourceW(
                update,
                int_resource(RT_GROUP_ICON),
                group_name.as_ptr(),
                LANG_NEUTRAL,
                group.as_ptr().cast(),
                u32::try_from(group.len()).unwrap_or(u32::MAX),
            ) != 0;
        if EndUpdateResourceW(update, i32::from(!ok)) == 0 || !ok {
            bail!("cannot set the icon of {}", executable.display());
        }
    }
    Ok(())
}

/// `MAKEINTRESOURCEW`: a resource id passed where Windows expects a name.
fn int_resource(id: u16) -> *const u16 {
    ptr::without_provenance(usize::from(id))
}

/// An `RT_GROUP_ICON` directory and its `RT_ICON` images with their resource ids.
type IconResources<'a> = (Vec<u8>, Vec<(u16, &'a [u8])>);

/// Splits an `.ico` file into resources: the `RT_GROUP_ICON` directory and one `RT_ICON` image
/// per size, numbered from `first_id`.
fn icon_resources(ico: &[u8], first_id: u16) -> anyhow::Result<IconResources<'_>> {
    let u16_at = |at: usize| -> Option<u16> {
        Some(u16::from_le_bytes(ico.get(at..at + 2)?.try_into().ok()?))
    };
    let u32_at = |at: usize| -> Option<u32> {
        Some(u32::from_le_bytes(ico.get(at..at + 4)?.try_into().ok()?))
    };
    let (Some(0), Some(1), Some(count)) = (u16_at(0), u16_at(2), u16_at(4)) else {
        bail!("not an .ico file");
    };
    // ICONDIR and GRPICONDIR share their 6-byte header; each 16-byte ICONDIRENTRY becomes a
    // 14-byte GRPICONDIRENTRY whose trailing image offset is replaced by a resource id.
    let mut group = ico[..6].to_vec();
    let mut images = Vec::new();
    for index in 0..count {
        let entry = 6 + usize::from(index) * 16;
        let id = first_id + index;
        let (Some(size), Some(offset)) = (u32_at(entry + 8), u32_at(entry + 12)) else {
            bail!("truncated .ico directory");
        };
        let start = usize::try_from(offset)?;
        let image = start
            .checked_add(usize::try_from(size)?)
            .and_then(|end| ico.get(start..end))
            .context("truncated .ico image")?;
        group.extend_from_slice(&ico[entry..entry + 12]);
        group.extend_from_slice(&id.to_le_bytes());
        images.push((id, image));
    }
    Ok((group, images))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An `.ico` with one 16x16 and one 32x32 image whose bytes are `image16` and `image32`.
    fn two_image_ico(image16: &[u8], image32: &[u8]) -> Vec<u8> {
        let mut ico = vec![0, 0, 1, 0, 2, 0];
        let mut offset = 6 + 2 * 16;
        for (side, image) in [(16u8, image16), (32, image32)] {
            ico.extend_from_slice(&[side, side, 0, 0, 1, 0, 32, 0]);
            ico.extend_from_slice(&u32::try_from(image.len()).unwrap().to_le_bytes());
            ico.extend_from_slice(&u32::try_from(offset).unwrap().to_le_bytes());
            offset += image.len();
        }
        ico.extend_from_slice(image16);
        ico.extend_from_slice(image32);
        ico
    }

    #[test]
    fn icon_resources_number_the_images_and_point_the_group_at_them() {
        let ico = two_image_ico(b"sixteen", b"thirty-two");

        let (group, images) = icon_resources(&ico, 101).unwrap();

        assert_eq!(
            images,
            vec![
                (101, b"sixteen".as_slice()),
                (102, b"thirty-two".as_slice())
            ]
        );
        let mut expected = vec![0, 0, 1, 0, 2, 0];
        expected.extend_from_slice(&[16, 16, 0, 0, 1, 0, 32, 0, 7, 0, 0, 0, 101, 0]);
        expected.extend_from_slice(&[32, 32, 0, 0, 1, 0, 32, 0, 10, 0, 0, 0, 102, 0]);
        assert_eq!(group, expected);
    }

    #[test]
    fn icon_resources_reject_truncated_files() {
        let ico = two_image_ico(b"sixteen", b"thirty-two");

        assert!(icon_resources(&ico[..ico.len() - 1], 101).is_err());
        assert!(icon_resources(&ico[..20], 101).is_err());
        assert!(icon_resources(b"PNG", 101).is_err());
    }

    #[test]
    fn the_app_icon_has_every_windows_size() {
        let (_, images) = icon_resources(ICON, FIRST_ICON_ID).unwrap();

        assert_eq!(images.len(), 7);
    }
}
