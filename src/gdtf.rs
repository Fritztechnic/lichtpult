//! GDTF-Import (`.gdtf` = ZIP mit `description.xml`): ein Fixture-Typ pro DMX-Modus.

use crate::fixture::{Attr, FixtureType};
use std::io::Read;

fn attr_of(name: &str) -> Option<Attr> {
    Some(match name.trim_end_matches(|c: char| c.is_ascii_digit()) {
        "Dimmer" => Attr::Dimmer,
        "Pan" => Attr::Pan,
        "Tilt" => Attr::Tilt,
        "ColorAdd_R" | "ColorRGB_Red" => Attr::Red,
        "ColorAdd_G" | "ColorRGB_Green" => Attr::Green,
        "ColorAdd_B" | "ColorRGB_Blue" => Attr::Blue,
        "ColorAdd_W" | "ColorRGB_White" => Attr::White,
        "ColorAdd_RY" | "ColorAdd_A" => Attr::Amber,
        "ColorAdd_UV" | "UV" => Attr::Uv,
        "Zoom" => Attr::Zoom,
        "Shutter" => Attr::Strobe,
        "Color" => Attr::ColorWheel,
        "Gobo" => Attr::Gobo,
        "Prism" => Attr::Prism,
        "Iris" => Attr::Iris,
        "Focus" => Attr::Focus,
        "Frost" => Attr::Frost,
        _ => return None,
    })
}

pub fn import_gdtf(bytes: &[u8]) -> Result<Vec<FixtureType>, String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut xml = String::new();
    zip.by_name("description.xml").map_err(|e| e.to_string())?.read_to_string(&mut xml).map_err(|e| e.to_string())?;
    parse(&xml)
}

pub fn parse(xml: &str) -> Result<Vec<FixtureType>, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;
    let ft = doc.descendants().find(|n| n.has_tag_name("FixtureType")).ok_or("kein FixtureType")?;
    let base = {
        let (m, n) = (ft.attribute("Manufacturer").unwrap_or(""), ft.attribute("Name").unwrap_or("GDTF"));
        if m.is_empty() { n.to_string() } else { format!("{m} {n}") }
    };
    let modes: Vec<_> = ft.descendants().filter(|n| n.has_tag_name("DMXMode")).collect();
    let types: Vec<_> = modes
        .iter()
        .filter_map(|m| {
            let mut channels: Vec<Option<Attr>> = Vec::new();
            let mut used = Vec::new();
            for c in m.descendants().filter(|n| n.has_tag_name("DMXChannel")) {
                let offs: Vec<usize> = c.attribute("Offset")?.split(',').filter_map(|o| o.trim().parse().ok()).collect();
                let Some(&first) = offs.first() else { continue };
                let attr = c
                    .descendants()
                    .find(|n| n.has_tag_name("LogicalChannel"))
                    .and_then(|l| l.attribute("Attribute"))
                    .and_then(attr_of)
                    .filter(|a| !used.contains(a));
                let need = offs.iter().max().copied().unwrap_or(first);
                if channels.len() < need {
                    channels.resize(need, None);
                }
                if let Some(a) = attr {
                    used.push(a);
                    channels[first - 1] = Some(a);
                    if let Some(&fine) = offs.get(1) {
                        channels[fine - 1] = match a {
                            Attr::Pan => Some(Attr::PanFine),
                            Attr::Tilt => Some(Attr::TiltFine),
                            _ => None,
                        };
                    }
                }
            }
            if channels.is_empty() {
                return None;
            }
            let name = if modes.len() > 1 { format!("{base} ({})", m.attribute("Name").unwrap_or("?")) } else { base.clone() };
            Some(FixtureType { name, channels })
        })
        .collect();
    if types.is_empty() { Err("keine DMX-Modi gefunden".into()) } else { Ok(types) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modes_with_fine_channels() {
        let xml = r#"<GDTF><FixtureType Name="Spot" Manufacturer="Acme"><DMXModes>
          <DMXMode Name="Std"><DMXChannels>
            <DMXChannel Offset="1,2" Geometry="b"><LogicalChannel Attribute="Pan"/></DMXChannel>
            <DMXChannel Offset="3" Geometry="b"><LogicalChannel Attribute="Dimmer"/></DMXChannel>
            <DMXChannel Offset="4" Geometry="b"><LogicalChannel Attribute="Gobo1"/></DMXChannel>
            <DMXChannel Offset="None" Geometry="b"><LogicalChannel Attribute="Dimmer"/></DMXChannel>
          </DMXChannels></DMXMode></DMXModes></FixtureType></GDTF>"#;
        let t = parse(xml).unwrap();
        assert_eq!(t[0].name, "Acme Spot");
        assert_eq!(t[0].channels, [Some(Attr::Pan), Some(Attr::PanFine), Some(Attr::Dimmer), Some(Attr::Gobo)]);
    }
}
