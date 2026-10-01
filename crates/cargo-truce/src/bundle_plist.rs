//! `Info.plist` contents for the macOS VST3 and AU v2 bundles.
//!
//! Shared by the packaging (`package/stage.rs`) and local-install
//! (`install/mod.rs`) paths so the two can't drift apart. Both stamp:
//!
//! - the per-format display name (`vst3_name` / `au_name` override,
//!   else `name`) - the same resolution the format wrappers apply at
//!   registration, so the plist and the binary report the same name.
//!   For AU v2 this matters most: hosts list the component under the
//!   plist's `AudioComponents` `name` before ever loading the binary.
//! - the plugin version (see [`crate::util::plugin_version`]) into
//!   `CFBundleVersion` / `CFBundleShortVersionString` and, packed, into
//!   the AU `AudioComponents` `version`, matching what the AU reports at
//!   runtime. Hosts key their AU validation cache on that value.
//!
//! Plain string builders (no filesystem access), so they're testable on
//! every host even though only macOS consumes them.

#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use crate::PluginDef;
use crate::commands::install::presets::resolved_name;
use crate::config::Config;
use crate::preset_codec::xml_escape;

/// `Info.plist` for a macOS `.vst3` bundle.
pub(crate) fn vst3_info_plist(p: &PluginDef, config: &Config, version: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>{exec_name}</string>
    <key>CFBundleIdentifier</key>
    <string>{vendor_id}.{bundle_id}</string>
    <key>CFBundleName</key>
    <string>{display_name}</string>
    <key>CFBundlePackageType</key>
    <string>BNDL</string>
    <key>CFBundleShortVersionString</key>
    <string>{version}</string>
    <key>CFBundleVersion</key>
    <string>{version}</string>
</dict>
</plist>"#,
        exec_name = xml_escape(&p.file_stem()),
        vendor_id = xml_escape(&config.vendor.id),
        bundle_id = xml_escape(&p.bundle_id),
        display_name = xml_escape(resolved_name(p.vst3_name.as_deref(), &p.name)),
        version = xml_escape(version),
    )
}

/// `Info.plist` for a macOS AU v2 `.component` bundle.
pub(crate) fn au2_info_plist(p: &PluginDef, config: &Config, version: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>{exec_name}</string>
    <key>CFBundleIdentifier</key>
    <string>{vendor_id}.{bundle_id}.component</string>
    <key>CFBundleName</key>
    <string>{display_name}</string>
    <key>CFBundlePackageType</key>
    <string>BNDL</string>
    <key>CFBundleShortVersionString</key>
    <string>{version}</string>
    <key>CFBundleVersion</key>
    <string>{version}</string>
    <key>AudioComponents</key>
    <array>
        <dict>
            <key>type</key>
            <string>{au_type}</string>
            <key>subtype</key>
            <string>{au_subtype}</string>
            <key>manufacturer</key>
            <string>{au_mfr}</string>
            <key>name</key>
            <string>{vendor}: {display_name}</string>
            <key>description</key>
            <string>{display_name}</string>
            <key>version</key>
            <integer>{au_version}</integer>
            <key>factoryFunction</key>
            <string>TruceAUFactory</string>
            <key>sandboxSafe</key>
            <true/>
            <key>tags</key>
            <array>
                <string>{au_tag}</string>
            </array>
        </dict>
    </array>
</dict>
</plist>"#,
        exec_name = xml_escape(&p.file_stem()),
        vendor_id = xml_escape(&config.vendor.id),
        bundle_id = xml_escape(&p.bundle_id),
        display_name = xml_escape(resolved_name(p.au_name.as_deref(), &p.name)),
        version = xml_escape(version),
        au_type = xml_escape(p.resolved_au_type()),
        au_subtype = xml_escape(p.resolved_fourcc()),
        au_mfr = xml_escape(&config.vendor.au_manufacturer),
        vendor = xml_escape(&config.vendor.name),
        au_version = truce_utils::au_version_u32(version),
        au_tag = xml_escape(&p.au_tag),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(plugin_extra: &str) -> Config {
        toml::from_str(&format!(
            r#"
[vendor]
name = "Acme Audio"
id = "com.acme"
au_manufacturer = "Acme"

[[plugin]]
name = "acme-tremolo"
bundle_id = "tremolo"
crate = "acme-tremolo"
category = "effect"
fourcc = "Trem"
{plugin_extra}
"#
        ))
        .expect("test truce.toml parses")
    }

    #[test]
    fn au2_uses_au_name_override() {
        let c = config(r#"au_name = "Acme Tremolo""#);
        let plist = au2_info_plist(&c.plugin[0], &c, "1.0.0");
        assert!(plist.contains("<string>Acme Audio: Acme Tremolo</string>"));
        assert!(
            plist.contains("<key>description</key>\n            <string>Acme Tremolo</string>")
        );
        assert!(plist.contains("<key>CFBundleName</key>\n    <string>Acme Tremolo</string>"));
        // The bundle file / executable keep following `name`.
        assert!(plist.contains("<string>acme-tremolo</string>"));
    }

    #[test]
    fn au2_falls_back_to_name() {
        let c = config("");
        let plist = au2_info_plist(&c.plugin[0], &c, "1.0.0");
        assert!(plist.contains("<string>Acme Audio: acme-tremolo</string>"));
    }

    #[test]
    fn au2_stamps_version() {
        let c = config("");
        let plist = au2_info_plist(&c.plugin[0], &c, "26.9.1");
        assert!(plist.contains("<key>CFBundleVersion</key>\n    <string>26.9.1</string>"));
        assert!(
            plist.contains("<key>CFBundleShortVersionString</key>\n    <string>26.9.1</string>")
        );
        // (26 << 16) | (9 << 8) | 1
        assert!(plist.contains("<integer>1706241</integer>"));
    }

    #[test]
    fn vst3_uses_vst3_name_and_version() {
        let c = config(r#"vst3_name = "Acme Tremolo""#);
        let plist = vst3_info_plist(&c.plugin[0], &c, "2.10.5");
        assert!(plist.contains("<key>CFBundleName</key>\n    <string>Acme Tremolo</string>"));
        assert!(plist.contains("<key>CFBundleVersion</key>\n    <string>2.10.5</string>"));
        assert!(plist.contains("<string>com.acme.tremolo</string>"));
    }

    #[test]
    fn escapes_xml_metachars() {
        let c = config(r#"au_name = "Tom & Jerry""#);
        let plist = au2_info_plist(&c.plugin[0], &c, "1.0.0");
        assert!(plist.contains("Tom &amp; Jerry"));
        assert!(!plist.contains("Tom & Jerry"));
    }
}
