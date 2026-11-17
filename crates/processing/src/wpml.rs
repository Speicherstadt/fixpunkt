//! Flugplan als KMZ für DJI Pilot 2 (DJI WPML, Wegpunkt-Mission): `wpmz/template.kml` und `wpmz/waylines.wpml`
//! in einem ZIP. Je Bahn zwei Wegpunkte; auf der Bahn löst die Drohne alle `photo_spacing` Meter ein Foto aus
//! (`multipleDistance`), am ersten Wegpunkt schwenkt die Kamera senkrecht nach unten.
//!
//! Elemente und Werte nach der WPML-Referenz der DJI Cloud API (github.com/dji-sdk/Cloud-API-Doc,
//! `docs/en/60.api-reference/00.dji-wpml`). Höhen relativ zum Startpunkt: dort starten, wo das Gelände im Mittel liegt.

use crate::crs::Utm;
use crate::flightplan::FlightPlan;
use anyhow::Result;
use std::fmt::Write as _;
use std::io::Write as _;

const NS: &str =
    r#"xmlns="http://www.opengis.net/kml/2.2" xmlns:wpml="http://www.dji.com/wpmz/1.0.2""#;

/// Drohne und Kamera als DJI-Kennungen (`droneEnumValue`, `droneSubEnumValue`, `payloadEnumValue`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drone {
    pub drone: u32,
    pub sub: u32,
    pub payload: u32,
}

/// Matrice 4E (Matrice-4-Serie 99, Untertyp 0, Kamera 88)
pub const M4E: Drone = Drone {
    drone: 99,
    sub: 0,
    payload: 88,
};

/// Mavic 3 Enterprise (77/0, Kamera 66) – laut Produktliste der Cloud API
pub const M3E: Drone = Drone {
    drone: 77,
    sub: 0,
    payload: 66,
};

/// Wegpunkte in Länge/Breite (WGS84; ETRS89 weicht in Europa nur Dezimeter ab)
pub fn waypoints(plan: &FlightPlan, utm: &Utm) -> Vec<(f64, f64)> {
    plan.lines
        .iter()
        .flat_map(|l| [l.start, l.end])
        .map(|[e, n]| {
            let (lat, lon) = utm.inverse(e, n);
            (lon, lat)
        })
        .collect()
}

fn mission_config(d: &Drone, speed: f64) -> String {
    format!(
        r#"<wpml:missionConfig>
<wpml:flyToWaylineMode>safely</wpml:flyToWaylineMode>
<wpml:finishAction>goHome</wpml:finishAction>
<wpml:exitOnRCLost>executeLostAction</wpml:exitOnRCLost>
<wpml:executeRCLostAction>goBack</wpml:executeRCLostAction>
<wpml:takeOffSecurityHeight>20</wpml:takeOffSecurityHeight>
<wpml:globalTransitionalSpeed>{speed:.1}</wpml:globalTransitionalSpeed>
<wpml:droneInfo><wpml:droneEnumValue>{}</wpml:droneEnumValue><wpml:droneSubEnumValue>{}</wpml:droneSubEnumValue></wpml:droneInfo>
<wpml:payloadInfo><wpml:payloadEnumValue>{}</wpml:payloadEnumValue><wpml:payloadPositionIndex>0</wpml:payloadPositionIndex></wpml:payloadInfo>
</wpml:missionConfig>"#,
        d.drone, d.sub, d.payload
    )
}

fn point(lon: f64, lat: f64) -> String {
    format!("<Point><coordinates>{lon:.9},{lat:.9}</coordinates></Point>")
}

/// `template.kml`: die Mission, wie Pilot 2 sie zum Bearbeiten öffnet
pub fn template_kml(plan: &FlightPlan, pts: &[(f64, f64)], d: &Drone, created_ms: i64) -> String {
    let h = plan.altitude;
    let mut marks = String::new();
    for (i, (lon, lat)) in pts.iter().enumerate() {
        let _ = writeln!(
            marks,
            "<Placemark>{}<wpml:index>{i}</wpml:index><wpml:ellipsoidHeight>{h:.2}</wpml:ellipsoidHeight>\
<wpml:height>{h:.2}</wpml:height><wpml:useGlobalHeight>1</wpml:useGlobalHeight><wpml:useGlobalSpeed>1</wpml:useGlobalSpeed>\
<wpml:useGlobalHeadingParam>1</wpml:useGlobalHeadingParam><wpml:useGlobalTurnParam>1</wpml:useGlobalTurnParam>\
<wpml:gimbalPitchAngle>-90</wpml:gimbalPitchAngle></Placemark>",
            point(*lon, *lat)
        );
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<kml {NS}>
<Document>
<wpml:author>Fixpunkt</wpml:author>
<wpml:createTime>{created_ms}</wpml:createTime>
<wpml:updateTime>{created_ms}</wpml:updateTime>
{}
<Folder>
<wpml:templateType>waypoint</wpml:templateType>
<wpml:templateId>0</wpml:templateId>
<wpml:waylineCoordinateSysParam><wpml:coordinateMode>WGS84</wpml:coordinateMode><wpml:heightMode>relativeToStartPoint</wpml:heightMode></wpml:waylineCoordinateSysParam>
<wpml:autoFlightSpeed>{:.1}</wpml:autoFlightSpeed>
<wpml:globalHeight>{h:.2}</wpml:globalHeight>
<wpml:gimbalPitchMode>usePointSetting</wpml:gimbalPitchMode>
<wpml:globalWaypointHeadingParam><wpml:waypointHeadingMode>followWayline</wpml:waypointHeadingMode></wpml:globalWaypointHeadingParam>
<wpml:globalWaypointTurnMode>toPointAndStopWithDiscontinuityCurvature</wpml:globalWaypointTurnMode>
<wpml:globalUseStraightLine>1</wpml:globalUseStraightLine>
{marks}</Folder>
</Document>
</kml>
"#,
        mission_config(d, plan.speed),
        plan.speed
    )
}

fn take_photo() -> &'static str {
    "<wpml:action><wpml:actionId>0</wpml:actionId><wpml:actionActuatorFunc>takePhoto</wpml:actionActuatorFunc>\
<wpml:actionActuatorFuncParam><wpml:payloadPositionIndex>0</wpml:payloadPositionIndex><wpml:fileSuffix>fixpunkt</wpml:fileSuffix>\
<wpml:useGlobalPayloadLensIndex>1</wpml:useGlobalPayloadLensIndex></wpml:actionActuatorFuncParam></wpml:action>"
}

fn gimbal_down() -> &'static str {
    "<wpml:action><wpml:actionId>0</wpml:actionId><wpml:actionActuatorFunc>gimbalRotate</wpml:actionActuatorFunc>\
<wpml:actionActuatorFuncParam><wpml:gimbalHeadingYawBase>north</wpml:gimbalHeadingYawBase>\
<wpml:gimbalRotateMode>absoluteAngle</wpml:gimbalRotateMode><wpml:gimbalPitchRotateEnable>1</wpml:gimbalPitchRotateEnable>\
<wpml:gimbalPitchRotateAngle>-90</wpml:gimbalPitchRotateAngle><wpml:gimbalRollRotateEnable>0</wpml:gimbalRollRotateEnable>\
<wpml:gimbalRollRotateAngle>0</wpml:gimbalRollRotateAngle><wpml:gimbalYawRotateEnable>0</wpml:gimbalYawRotateEnable>\
<wpml:gimbalYawRotateAngle>0</wpml:gimbalYawRotateAngle><wpml:gimbalRotateTimeEnable>0</wpml:gimbalRotateTimeEnable>\
<wpml:gimbalRotateTime>0</wpml:gimbalRotateTime><wpml:payloadPositionIndex>0</wpml:payloadPositionIndex>\
</wpml:actionActuatorFuncParam></wpml:action>"
}

/// Aktionsgruppe: Auslöser von Wegpunkt `from` bis `to`
fn action_group(id: usize, from: usize, to: usize, trigger: &str, action: &str) -> String {
    format!(
        "<wpml:actionGroup><wpml:actionGroupId>{id}</wpml:actionGroupId><wpml:actionGroupStartIndex>{from}</wpml:actionGroupStartIndex>\
<wpml:actionGroupEndIndex>{to}</wpml:actionGroupEndIndex><wpml:actionGroupMode>sequence</wpml:actionGroupMode>\
<wpml:actionTrigger>{trigger}</wpml:actionTrigger>{action}</wpml:actionGroup>"
    )
}

/// `waylines.wpml`: was die Drohne ausführt
pub fn waylines_wpml(plan: &FlightPlan, pts: &[(f64, f64)], d: &Drone) -> String {
    let (h, v) = (plan.altitude, plan.speed);
    let mut marks = String::new();
    let mut group = 0;
    for (i, (lon, lat)) in pts.iter().enumerate() {
        let mut actions = String::new();
        if i == 0 {
            actions += &action_group(
                group,
                0,
                0,
                "<wpml:actionTriggerType>reachPoint</wpml:actionTriggerType>",
                gimbal_down(),
            );
            group += 1;
        }
        if i % 2 == 0 {
            let trigger = format!(
                "<wpml:actionTriggerType>multipleDistance</wpml:actionTriggerType>\
<wpml:actionTriggerParam>{:.2}</wpml:actionTriggerParam>",
                plan.photo_spacing
            );
            actions += &action_group(group, i, i + 1, &trigger, take_photo());
            group += 1;
        }
        let _ = writeln!(
            marks,
            "<Placemark>{}<wpml:index>{i}</wpml:index><wpml:executeHeight>{h:.2}</wpml:executeHeight>\
<wpml:waypointSpeed>{v:.1}</wpml:waypointSpeed>\
<wpml:waypointHeadingParam><wpml:waypointHeadingMode>followWayline</wpml:waypointHeadingMode></wpml:waypointHeadingParam>\
<wpml:waypointTurnParam><wpml:waypointTurnMode>toPointAndStopWithDiscontinuityCurvature</wpml:waypointTurnMode>\
<wpml:waypointTurnDampingDist>0</wpml:waypointTurnDampingDist></wpml:waypointTurnParam>\
<wpml:useStraightLine>1</wpml:useStraightLine>{actions}</Placemark>",
            point(*lon, *lat)
        );
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<kml {NS}>
<Document>
{}
<Folder>
<wpml:templateId>0</wpml:templateId>
<wpml:executeHeightMode>relativeToStartPoint</wpml:executeHeightMode>
<wpml:waylineId>0</wpml:waylineId>
<wpml:distance>{:.1}</wpml:distance>
<wpml:duration>{:.1}</wpml:duration>
<wpml:autoFlightSpeed>{v:.1}</wpml:autoFlightSpeed>
{marks}</Folder>
</Document>
</kml>
"#,
        mission_config(d, v),
        plan.length,
        plan.duration
    )
}

/// KMZ (ZIP) mit `wpmz/template.kml` und `wpmz/waylines.wpml`
pub fn kmz(plan: &FlightPlan, utm: &Utm, d: &Drone, created_ms: i64) -> Result<Vec<u8>> {
    let pts = waypoints(plan, utm);
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("wpmz/template.kml", opts)?;
    zip.write_all(template_kml(plan, &pts, d, created_ms).as_bytes())?;
    zip.start_file("wpmz/waylines.wpml", opts)?;
    zip.write_all(waylines_wpml(plan, &pts, d).as_bytes())?;
    Ok(zip.finish()?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flightplan::{self, FlightParams};
    use crate::geom::Polygon;
    use std::io::Read;

    fn sample() -> FlightPlan {
        let (x0, y0) = Utm::new(32).unwrap().forward(48.0, 9.0); // Mittelmeridian der Zone 32
        let field = Polygon::rect(x0, y0, x0 + 100.0, y0 + 60.0);
        flightplan::plan(&field, &flightplan::M4E, &FlightParams::default()).unwrap()
    }

    #[test]
    fn kmz_contains_both_files_with_all_waypoints() {
        let plan = sample();
        let bytes = kmz(&plan, &Utm::new(32).unwrap(), &M4E, 1_700_000_000_000).unwrap();
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut read = |name: &str| {
            let mut s = String::new();
            z.by_name(name).unwrap().read_to_string(&mut s).unwrap();
            s
        };
        let (t, w) = (read("wpmz/template.kml"), read("wpmz/waylines.wpml"));
        // 5 Bahnen → 10 Wegpunkte in beiden Dateien
        assert_eq!(t.matches("<Placemark>").count(), 10);
        assert_eq!(w.matches("<Placemark>").count(), 10);
        assert!(t.contains("<wpml:droneEnumValue>99</wpml:droneEnumValue>"));
        assert!(t.contains("<wpml:globalHeight>75.02</wpml:globalHeight>"));
        assert!(w.contains("<wpml:executeHeightMode>relativeToStartPoint</wpml:executeHeightMode>"));
        // je Bahn eine Fotogruppe alle 15,82 m, dazu die Kamera nach unten
        assert_eq!(w.matches("multipleDistance").count(), 5);
        assert_eq!(w.matches("<wpml:actionTriggerParam>15.82<").count(), 5);
        assert_eq!(w.matches("gimbalRotate<").count(), 1);
        assert!(w.contains("<wpml:actionGroupStartIndex>8</wpml:actionGroupStartIndex><wpml:actionGroupEndIndex>9<"));
        // gültiges XML: jedes geöffnete Element wird geschlossen
        for doc in [&t, &w] {
            assert_eq!(
                doc.matches("<wpml:actionGroup>").count(),
                doc.matches("</wpml:actionGroup>").count()
            );
            assert!(doc.trim_end().ends_with("</kml>"));
        }
    }

    #[test]
    fn waypoints_in_degrees() {
        let plan = sample();
        let pts = waypoints(&plan, &Utm::new(32).unwrap());
        // Rechteck beginnt bei 48° N, 9° O: mittlere Bahn bei +50 m Ost, ein Grad Länge ≈ 74,6 km bei 48°
        let (lon, lat) = pts[4];
        assert!((lon - (9.0 + 50.0 / 74_625.0)).abs() < 2e-6, "{lon}");
        assert!((lat - 48.0).abs() < 0.001);
        // die mittlere Bahn verläuft genau Nord–Süd: gleiche Länge an Anfang und Ende (auf dem Mittelmeridian)
        assert!((pts[4].0 - pts[5].0).abs() < 1e-7);
    }
}
