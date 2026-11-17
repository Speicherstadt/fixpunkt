//! Umrechnung zwischen festem Raummaß (wie gemessen), losem Raummaß (wie geladen), Tonnen und LKW-Ladungen.
//!
//! - lose m³ = fest m³ × Auflockerungsfaktor
//! - t = fest m³ × Dichte (t/m³, fest)
//! - Ladungen = t ÷ Nutzlast, aufgerundet auf ganze Fahrten

use serde::{Deserialize, Serialize};

/// Kennwerte eines Materials; fehlende Werte heißen „nicht umrechenbar“
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Props {
    /// t/m³ im gewachsenen (festen) Zustand
    pub density: Option<f64>,
    /// lose ÷ fest, z. B. 1,25
    pub bulking: Option<f64>,
}

/// Eine Menge in allen Einheiten
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quantities {
    pub bank_m3: f64,
    pub loose_m3: Option<f64>,
    pub tonnes: Option<f64>,
    /// ganze LKW-Fahrten
    pub loads: Option<f64>,
}

pub fn quantities(bank_m3: f64, p: Props, truck_t: Option<f64>) -> Quantities {
    let tonnes = p.density.map(|d| bank_m3 * d);
    Quantities {
        bank_m3,
        loose_m3: p.bulking.map(|b| bank_m3 * b),
        tonnes,
        loads: loads(tonnes, truck_t),
    }
}

fn loads(tonnes: Option<f64>, truck_t: Option<f64>) -> Option<f64> {
    let (t, cap) = (tonnes?, truck_t.filter(|c| *c > 0.0)?);
    // 1e-9 gegen Rundungsfehler: 300 t / 15 t sind 20 Fahrten, nicht 21
    Some((t.abs() / cap - 1e-9).ceil().max(0.0))
}

/// Menge in der Preiseinheit eines Bereichs: `m3` (fest), `m3_loose` oder `t`
pub fn in_unit(q: &Quantities, unit: &str) -> Option<f64> {
    match unit {
        "m3" => Some(q.bank_m3),
        "m3_loose" => q.loose_m3,
        "t" => q.tonnes,
        _ => None,
    }
}

/// Menge eines Lieferscheins in festen m³: `t`, `m3` (lose, wie geladen) oder `loads` (volle Ladungen)
pub fn to_bank(quantity: f64, unit: &str, p: Props, truck_t: Option<f64>) -> Option<f64> {
    match unit {
        "t" => p.density.filter(|d| *d > 0.0).map(|d| quantity / d),
        "m3" => p.bulking.filter(|b| *b > 0.0).map(|b| quantity / b),
        "loads" => to_bank(quantity * truck_t.filter(|c| *c > 0.0)?, "t", p, None),
        _ => None,
    }
}

/// Einheiten eines Lieferscheins
pub const DELIVERY_UNITS: &[&str] = &["t", "m3", "loads"];

#[cfg(test)]
mod tests {
    use super::*;

    const LEHM: Props = Props {
        density: Some(2.0),
        bulking: Some(1.25),
    };

    #[test]
    fn all_units_of_a_volume() {
        // 800 m³ fest Lehm: 1.000 m³ lose, 1.600 t, mit 15-t-Kipper 106,7 → 107 Fahrten
        let q = quantities(800.0, LEHM, Some(15.0));
        assert_eq!(q.loose_m3, Some(1000.0));
        assert_eq!(q.tonnes, Some(1600.0));
        assert_eq!(q.loads, Some(107.0));
        // genau aufgehend: 300 t mit 15 t → 20 Fahrten
        assert_eq!(quantities(150.0, LEHM, Some(15.0)).loads, Some(20.0));
        assert_eq!(in_unit(&q, "m3"), Some(800.0));
        assert_eq!(in_unit(&q, "m3_loose"), Some(1000.0));
        assert_eq!(in_unit(&q, "t"), Some(1600.0));
        assert_eq!(in_unit(&q, "kg"), None);
    }

    #[test]
    fn missing_properties_are_not_guessed() {
        let q = quantities(100.0, Props::default(), Some(15.0));
        assert_eq!((q.loose_m3, q.tonnes, q.loads), (None, None, None));
        assert_eq!(in_unit(&q, "t"), None);
        // ohne Nutzlast keine Fahrten
        assert_eq!(quantities(100.0, LEHM, None).loads, None);
        assert_eq!(to_bank(10.0, "loads", LEHM, None), None);
    }

    #[test]
    fn delivery_back_to_bank_volume() {
        // 1.600 t Lehm sind 800 m³ fest; 1.000 m³ lose ebenso; 100 Ladungen à 16 t ebenso
        assert_eq!(to_bank(1600.0, "t", LEHM, None), Some(800.0));
        assert_eq!(to_bank(1000.0, "m3", LEHM, None), Some(800.0));
        assert_eq!(to_bank(100.0, "loads", LEHM, Some(16.0)), Some(800.0));
        assert_eq!(to_bank(1.0, "Sack", LEHM, Some(16.0)), None);
    }
}
