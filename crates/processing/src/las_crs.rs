//! Bezugssystem in LAS/LAZ: EPSG aus dem GeoKeyDirectory oder WKT lesen, GeoKeys schreiben.

use anyhow::{Context, Result};
use std::path::Path;

/// VLR-Nummer des GeoTIFF-GeoKeyDirectory in LAS
pub(crate) const GEOKEY_DIRECTORY: u16 = 34735;
/// VLR-Nummer des Bezugssystems als WKT (LAS 1.4, PDAL)
const WKT: u16 = 2112;
/// GeoKey für das projizierte Bezugssystem
const PROJECTED_CS: u16 = 3072;

/// GeoKeyDirectory mit Modelltyp „projiziert“ und dem EPSG-Code
pub(crate) fn geokeys(epsg: u16) -> Vec<u8> {
    let keys: [u16; 16] = [
        1,
        1,
        0,
        3, // Version 1.1.0, drei Schlüssel
        1024,
        0,
        1,
        1, // GTModelType: projiziert
        1025,
        0,
        1,
        1, // GTRasterType: PixelIsArea
        PROJECTED_CS,
        0,
        1,
        epsg,
    ];
    keys.iter().flat_map(|k| k.to_le_bytes()).collect()
}

/// EPSG des projizierten Bezugssystems aus einem GeoKeyDirectory (u16, little endian)
pub fn epsg_from_geokeys(data: &[u8]) -> Option<u32> {
    let v: Vec<u16> = data
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes(*b))
        .collect();
    let n = usize::from(*v.get(3)?);
    v.get(4..4 + 4 * n)?
        .as_chunks::<4>()
        .0
        .iter()
        .find(|k| k[0] == PROJECTED_CS && k[1] == 0)
        .map(|k| u32::from(k[3]))
        .filter(|&c| c != 0 && c != 32767)
}

/// EPSG aus WKT: der letzte `AUTHORITY["EPSG","…"]` bzw. `ID["EPSG",…]` gehört zum Bezugssystem selbst
pub fn epsg_from_wkt(wkt: &str) -> Option<u32> {
    let digits = |s: &str| -> Option<u32> {
        let t = s.trim_start_matches(|c: char| c == '"' || c == ',' || c.is_whitespace());
        let end = t.find(|c: char| !c.is_ascii_digit()).unwrap_or(t.len());
        t[..end].parse().ok()
    };
    let a = wkt
        .rfind("AUTHORITY[\"EPSG\"")
        .map(|i| &wkt[i + "AUTHORITY[\"EPSG\"".len()..]);
    let b = wkt
        .rfind("ID[\"EPSG\"")
        .map(|i| &wkt[i + "ID[\"EPSG\"".len()..]);
    match (a, b) {
        (Some(x), Some(y)) => digits(if x.len() < y.len() { x } else { y }),
        (x, y) => digits(x.or(y)?),
    }
}

/// Bezugssystem einer LAS/LAZ-Datei (GeoKeys oder WKT); `None`, wenn keins angegeben ist
pub fn las_epsg(path: &Path) -> Result<Option<u32>> {
    let reader =
        las::Reader::from_path(path).with_context(|| format!("{} nicht lesbar", path.display()))?;
    let header = reader.header();
    let mut found = None;
    for v in header.all_vlrs() {
        if v.user_id.trim_end_matches('\0') != "LASF_Projection" {
            continue;
        }
        let code = match v.record_id {
            GEOKEY_DIRECTORY => epsg_from_geokeys(&v.data),
            WKT => epsg_from_wkt(&String::from_utf8_lossy(&v.data)),
            _ => None,
        };
        found = found.or(code);
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pointcloud::{Point3, PointCloud, ReadOptions};

    #[test]
    fn epsg_from_las_geokeys_and_wkt() {
        let dir = std::env::temp_dir().join(format!("fixpunkt-epsg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let c = PointCloud::new(vec![
            Point3 {
                x: 33_500_000.0,
                y: 5_500_000.0,
                z: 100.0,
            },
            Point3 {
                x: 33_500_010.0,
                y: 5_500_010.0,
                z: 101.0,
            },
        ]);
        let with = dir.join("odm.laz");
        c.write_las_georef(&with, None, Some(32633)).unwrap();
        assert_eq!(las_epsg(&with).unwrap(), Some(32633));
        let without = dir.join("plain.las");
        c.write_las(&without).unwrap();
        assert_eq!(las_epsg(&without).unwrap(), None);
        // Punkte unverändert
        assert_eq!(
            PointCloud::read(&with, ReadOptions::default())
                .unwrap()
                .len(),
            2
        );

        // WKT wie von PDAL: das äußerste AUTHORITY steht am Ende
        let wkt = r#"PROJCS["WGS 84 / UTM zone 32N",GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],AUTHORITY["EPSG","4326"]],PROJECTION["Transverse_Mercator"],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AUTHORITY["EPSG","32632"]]"#;
        assert_eq!(epsg_from_wkt(wkt), Some(32632));
        let wkt2 = r#"PROJCRS["ETRS89 / UTM zone 32N",BASEGEOGCRS["ETRS89",ID["EPSG",4258]],ID["EPSG",25832]]"#;
        assert_eq!(epsg_from_wkt(wkt2), Some(25832));
        assert_eq!(epsg_from_wkt("LOCAL_CS[\"x\"]"), None);
        assert_eq!(epsg_from_geokeys(&[1, 0]), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
