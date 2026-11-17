//! Bericht als PDF (A4 hoch): Helvetica aus den 14 Standardschriften, nichts eingebettet außer der Karte.
//! Gestaltung wie die Web-UI: Zink-Grautöne, Orange für Abtrag, Blau für Auftrag.

use super::doc::{Block, Doc, Kpi, Map, Table, Tone};
use super::text::{self, Weight};
use flate2::{write::ZlibEncoder, Compression};
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref, Str, TextStr};
use std::io::Write;

const PAGE_W: f32 = 595.28;
const PAGE_H: f32 = 841.89;
const MARGIN: f32 = 48.0;
const CONTENT_W: f32 = PAGE_W - 2.0 * MARGIN;
/// unterhalb dieser Höhe steht nur die Fußzeile
const BOTTOM: f32 = MARGIN + 16.0;
const MAP_MAX_H: f32 = 380.0;
/// kleiner wird die Karte nicht, um noch auf die Seite zu passen; sonst neue Seite
const MAP_MIN_H: f32 = 220.0;
const ROW_H: f32 = 18.0;

type Rgb = (u8, u8, u8);
const TEXT: Rgb = (24, 24, 27);
const MUTED: Rgb = (113, 113, 122);
const BORDER: Rgb = (228, 228, 231);
const CUT: Rgb = (180, 83, 9);
const FILL: Rgb = (29, 78, 216);
const OK: Rgb = (21, 128, 61);
const WHITE: Rgb = (255, 255, 255);

const FOOTER: &str =
    "Erstellt mit Fixpunkt. Kein amtliches Aufmaß – vor der Abrechnung plausibilisieren.";

struct Image {
    cols: usize,
    rows: usize,
    zlib: Vec<u8>,
}

/// Seiten, Schreibposition (y von unten, wie im PDF) und Bilder
struct Painter {
    pages: Vec<Content>,
    /// Seite, auf die gezeichnet wird
    cur: usize,
    y: f32,
    images: Vec<Image>,
}

fn font(w: Weight) -> Name<'static> {
    match w {
        Weight::Regular => Name(b"F1"),
        Weight::Bold => Name(b"F2"),
    }
}

fn image_name(i: usize) -> String {
    format!("Im{i}")
}

fn rgb(c: Rgb) -> (f32, f32, f32) {
    (
        f32::from(c.0) / 255.0,
        f32::from(c.1) / 255.0,
        f32::from(c.2) / 255.0,
    )
}

fn zlib(data: &[u8]) -> Vec<u8> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(data).expect("in den Speicher schreiben");
    e.finish().expect("in den Speicher schreiben")
}

impl Painter {
    fn new() -> Self {
        let mut p = Painter {
            pages: Vec::new(),
            cur: 0,
            y: 0.0,
            images: Vec::new(),
        };
        p.new_page();
        p
    }

    fn new_page(&mut self) {
        self.pages.push(Content::new());
        self.cur = self.pages.len() - 1;
        self.y = PAGE_H - MARGIN;
    }

    /// neue Seite, wenn `h` pt nicht mehr passen; true, wenn umgebrochen wurde
    fn ensure(&mut self, h: f32) -> bool {
        if self.y - h < BOTTOM {
            self.new_page();
            return true;
        }
        false
    }

    fn c(&mut self) -> &mut Content {
        &mut self.pages[self.cur]
    }

    fn text(&mut self, x: f32, y: f32, s: &str, w: Weight, size: f32, color: Rgb) {
        let (r, g, b) = rgb(color);
        let bytes = text::encode(s);
        let c = self.c();
        c.set_fill_rgb(r, g, b);
        c.begin_text();
        c.set_font(font(w), size);
        c.next_line(x, y);
        c.show(Str(&bytes));
        c.end_text();
    }

    fn text_right(&mut self, right: f32, y: f32, s: &str, w: Weight, size: f32, color: Rgb) {
        let x = right - text::width(s, w, size);
        self.text(x, y, s, w, size, color);
    }

    fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Rgb) {
        let (r, g, b) = rgb(color);
        self.c()
            .set_fill_rgb(r, g, b)
            .rect(x, y, w, h)
            .fill_nonzero();
    }

    fn stroke_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Rgb, width: f32) {
        let (r, g, b) = rgb(color);
        self.c()
            .set_stroke_rgb(r, g, b)
            .set_line_width(width)
            .rect(x, y, w, h)
            .stroke();
    }

    fn line(&mut self, x0: f32, x1: f32, y: f32, color: Rgb, width: f32) {
        let (r, g, b) = rgb(color);
        self.c()
            .set_stroke_rgb(r, g, b)
            .set_line_width(width)
            .move_to(x0, y)
            .line_to(x1, y)
            .stroke();
    }

    /// RGB-Bild `cols × rows` in das Rechteck `[x, y, Breite, Höhe]` (y unten)
    fn image(&mut self, cols: usize, rows: usize, rgb: &[u8], [x, y, w, h]: [f32; 4]) {
        self.images.push(Image {
            cols,
            rows,
            zlib: zlib(rgb),
        });
        let name = image_name(self.images.len() - 1);
        self.c()
            .save_state()
            .transform([w, 0.0, 0.0, h, x, y])
            .x_object(Name(name.as_bytes()))
            .restore_state();
    }

    fn header(&mut self, doc: &Doc) {
        let top = self.y;
        self.text(MARGIN, top - 8.0, "FIXPUNKT", Weight::Bold, 8.0, MUTED);
        self.text(MARGIN, top - 34.0, &doc.title, Weight::Bold, 20.0, TEXT);
        let sub = text::fit(&doc.subtitle, Weight::Regular, 11.0, CONTENT_W);
        self.text(MARGIN, top - 52.0, &sub, Weight::Regular, 11.0, MUTED);
        self.y = top - 72.0;
        let half = CONTENT_W / 2.0;
        for pair in doc.meta.chunks(2) {
            for (i, (k, v)) in pair.iter().enumerate() {
                let x = MARGIN + i as f32 * half;
                let y = self.y;
                self.text(x, y - 8.0, k, Weight::Regular, 8.0, MUTED);
                let v = text::fit(v, Weight::Regular, 9.5, half - 12.0);
                self.text(x, y - 20.0, &v, Weight::Regular, 9.5, TEXT);
            }
            self.y -= 28.0;
        }
        self.y -= 4.0;
        self.line(MARGIN, PAGE_W - MARGIN, self.y, BORDER, 0.75);
        self.y -= 16.0;
    }

    fn kpis(&mut self, kpis: &[Kpi]) {
        if kpis.is_empty() {
            return;
        }
        let gap = 10.0;
        let n = kpis.len() as f32;
        let w = (CONTENT_W - gap * (n - 1.0)) / n;
        let h = 52.0;
        self.ensure(h + 16.0);
        let y = self.y - h;
        for (i, k) in kpis.iter().enumerate() {
            let x = MARGIN + i as f32 * (w + gap);
            self.stroke_rect(x, y, w, h, BORDER, 0.75);
            let label = text::fit(&k.label, Weight::Regular, 8.5, w - 20.0);
            self.text(x + 10.0, y + h - 17.0, &label, Weight::Regular, 8.5, MUTED);
            let color = match k.tone {
                Tone::Neutral => TEXT,
                Tone::Removed => CUT,
                Tone::Added => FILL,
                Tone::Ok => OK,
            };
            let value = text::fit(&k.value, Weight::Bold, 15.0, w - 20.0);
            self.text(x + 10.0, y + 12.0, &value, Weight::Bold, 15.0, color);
        }
        self.y = y - 16.0;
    }

    /// Überschrift nie allein unten auf der Seite: `next` ist der Platz, den der Anfang des folgenden
    /// Blocks braucht
    fn heading(&mut self, s: &str, next: f32) {
        self.ensure(24.0 + next);
        self.y -= 14.0;
        self.text(MARGIN, self.y, s, Weight::Bold, 12.0, TEXT);
        self.y -= 10.0;
    }

    fn paragraph(&mut self, s: &str, color: Rgb, indent: f32) {
        for line in text::wrap(s, Weight::Regular, 9.0, CONTENT_W - 2.0 * indent) {
            self.ensure(13.0);
            self.y -= 12.0;
            self.text(MARGIN + indent, self.y, &line, Weight::Regular, 9.0, color);
        }
        self.y -= 6.0;
    }

    fn note(&mut self, s: &str) {
        let lines = text::wrap(s, Weight::Regular, 9.0, CONTENT_W - 20.0);
        let h = lines.len() as f32 * 12.0 + 14.0;
        self.ensure(h + 8.0);
        let top = self.y - 4.0;
        self.stroke_rect(MARGIN, top - h, CONTENT_W, h, CUT, 1.0);
        let mut y = top - 4.0;
        for line in lines {
            y -= 12.0;
            self.text(MARGIN + 10.0, y, &line, Weight::Regular, 9.0, TEXT);
        }
        self.y = top - h - 10.0;
    }

    fn table(&mut self, t: &Table) {
        let total_share: f32 = t.columns.iter().map(|c| c.share).sum();
        let widths: Vec<f32> = t
            .columns
            .iter()
            .map(|c| c.share / total_share * CONTENT_W)
            .collect();
        let row = |p: &mut Painter, cells: &[String], w: Weight, color: Rgb| {
            let mut x = MARGIN;
            let y = p.y - 12.5;
            for ((cell, col), width) in cells.iter().zip(&t.columns).zip(&widths) {
                let s = text::fit(cell, w, 9.0, width - 8.0);
                if col.right {
                    p.text_right(x + width - 4.0, y, &s, w, 9.0, color);
                } else {
                    p.text(x + 4.0, y, &s, w, 9.0, color);
                }
                x += width;
            }
            p.y -= ROW_H;
        };
        let head: Vec<String> = t.columns.iter().map(|c| c.title.clone()).collect();
        let draw_head = |p: &mut Painter| {
            row(p, &head, Weight::Bold, MUTED);
            p.line(MARGIN, PAGE_W - MARGIN, p.y, BORDER, 0.75);
        };
        self.ensure(3.0 * ROW_H);
        draw_head(self);
        for cells in &t.rows {
            if self.ensure(ROW_H) {
                draw_head(self);
            }
            row(self, cells, Weight::Regular, TEXT);
            self.line(MARGIN, PAGE_W - MARGIN, self.y, BORDER, 0.5);
        }
        if let Some(total) = &t.total {
            if self.ensure(ROW_H) {
                draw_head(self);
            }
            self.line(MARGIN, PAGE_W - MARGIN, self.y, TEXT, 0.75);
            row(self, total, Weight::Bold, TEXT);
        }
        self.y -= 10.0;
    }

    fn map(&mut self, m: &Map) {
        let room = self.y - BOTTOM - MAP_EXTRA - 4.0;
        let max_h = if room >= MAP_MIN_H.min(map_size(m, MAP_MAX_H).1) {
            room.min(MAP_MAX_H)
        } else {
            self.new_page();
            MAP_MAX_H
        };
        let (w, h) = map_size(m, max_h);
        let x = MARGIN + (CONTENT_W - w) / 2.0;
        let top = self.y - 4.0;
        self.image(m.cols, m.rows, &m.rgb, [x, top - h, w, h]);
        self.stroke_rect(x, top - h, w, h, BORDER, 0.75);
        self.outlines(m, x, top, w, h);
        self.y = top - h - 14.0;
        self.legend(m, x, w);
        self.paragraph(&m.caption, MUTED, 0.0);
    }

    fn outlines(&mut self, m: &Map, x: f32, top: f32, w: f32, h: f32) {
        let at = |p: &[f64; 2]| (x + p[0] as f32 * w, top - p[1] as f32 * h);
        for (name, ring) in &m.outlines {
            if ring.len() < 3 {
                continue;
            }
            let (r, g, b) = rgb(TEXT);
            let (x0, y0) = at(&ring[0]);
            let c = self.c();
            c.set_stroke_rgb(r, g, b)
                .set_line_width(1.0)
                .move_to(x0, y0);
            for p in &ring[1..] {
                let (px, py) = at(p);
                c.line_to(px, py);
            }
            c.close_path().stroke();
            let n = ring.len() as f64;
            let (cx, cy) = at(&[
                ring.iter().map(|p| p[0]).sum::<f64>() / n,
                ring.iter().map(|p| p[1]).sum::<f64>() / n,
            ]);
            let label = text::fit(name, Weight::Bold, 8.0, w / 3.0);
            let lw = text::width(&label, Weight::Bold, 8.0);
            self.fill_rect(cx - lw / 2.0 - 3.0, cy - 4.0, lw + 6.0, 12.0, WHITE);
            self.text(cx - lw / 2.0, cy - 0.5, &label, Weight::Bold, 8.0, TEXT);
        }
    }

    /// Farbverlauf mit drei Beschriftungen links, darunter rechts die Maßstabsleiste
    fn legend(&mut self, m: &Map, x: f32, w: f32) {
        let bar_w = 160.0_f32.min(w / 2.0);
        let rgb: Vec<u8> = m.legend.iter().flatten().copied().collect();
        self.image(m.legend.len(), 1, &rgb, [x, self.y, bar_w, 7.0]);
        let y = self.y - 11.0;
        let [lo, mid, hi] = &m.legend_labels;
        self.text(x, y, lo, Weight::Regular, 7.5, MUTED);
        let mw = text::width(mid, Weight::Regular, 7.5);
        self.text(x + (bar_w - mw) / 2.0, y, mid, Weight::Regular, 7.5, MUTED);
        self.text_right(x + bar_w, y, hi, Weight::Regular, 7.5, MUTED);
        let (meters, len) = scale_bar(m.width_m, w);
        let right = x + w;
        let bar_y = self.y;
        self.fill_rect(right - len, bar_y, len, 3.0, TEXT);
        self.fill_rect(
            right - len / 2.0,
            bar_y + 0.75,
            len / 2.0 - 0.75,
            1.5,
            WHITE,
        );
        let label = format!("{} m", text::number(meters, 0));
        self.text_right(right, y, &label, Weight::Regular, 7.5, MUTED);
        self.y = y - 8.0;
    }

    fn footers(&mut self) {
        let n = self.pages.len();
        for i in 0..n {
            self.cur = i;
            self.line(MARGIN, PAGE_W - MARGIN, MARGIN, BORDER, 0.5);
            self.text(MARGIN, MARGIN - 12.0, FOOTER, Weight::Regular, 7.5, MUTED);
            let s = format!("Seite {} von {n}", i + 1);
            self.text_right(
                PAGE_W - MARGIN,
                MARGIN - 12.0,
                &s,
                Weight::Regular,
                7.5,
                MUTED,
            );
        }
    }
}

/// Platz unter der Karte für Legende und eine Zeile Bildunterschrift
const MAP_EXTRA: f32 = 56.0;

/// Größe der Karte auf der Seite in pt: volle Breite oder höchstens `max_h` hoch
fn map_size(m: &Map, max_h: f32) -> (f32, f32) {
    let scale = (CONTENT_W / m.cols as f32).min(max_h / m.rows as f32);
    (m.cols as f32 * scale, m.rows as f32 * scale)
}

/// Platz, den der Anfang eines Blocks mindestens braucht (Tabelle: Kopf und zwei Zeilen)
fn lead(b: Option<&Block>) -> f32 {
    match b {
        Some(Block::Map(m)) => map_size(m, MAP_MIN_H).1 + MAP_EXTRA,
        Some(Block::Table(_)) => 3.0 * ROW_H,
        Some(Block::Note(_)) => 40.0,
        Some(Block::Text(_)) => 2.0 * 12.0,
        Some(Block::Heading(_)) | None => 0.0,
    }
}

/// Länge der Maßstabsleiste: runde Meterzahl um ein Fünftel der Bildbreite; (Meter, Länge in pt)
pub fn scale_bar(width_m: f64, width_pt: f32) -> (f64, f32) {
    let target = (width_m / 5.0).max(1e-3);
    let pow = 10f64.powf(target.log10().floor());
    let step = [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|s| s * pow)
        .rfind(|s| *s <= target)
        .unwrap_or(pow);
    (step, (step / width_m) as f32 * width_pt)
}

/// Bericht als PDF-Datei
pub fn render(doc: &Doc) -> Vec<u8> {
    let mut p = Painter::new();
    p.header(doc);
    p.kpis(&doc.kpis);
    for (i, b) in doc.blocks.iter().enumerate() {
        match b {
            Block::Heading(s) => p.heading(s, lead(doc.blocks.get(i + 1))),
            Block::Text(s) => p.paragraph(s, TEXT, 0.0),
            Block::Note(s) => p.note(s),
            Block::Table(t) => p.table(t),
            Block::Map(m) => p.map(m),
        }
    }
    p.footers();
    assemble(doc, p)
}

fn assemble(doc: &Doc, p: Painter) -> Vec<u8> {
    let mut pdf = Pdf::new();
    let mut next = 1;
    let mut alloc = || {
        next += 1;
        Ref::new(next - 1)
    };
    let (catalog, tree, regular, bold, info) = (alloc(), alloc(), alloc(), alloc(), alloc());
    let images: Vec<Ref> = p.images.iter().map(|_| alloc()).collect();
    let pages: Vec<(Ref, Ref)> = p.pages.iter().map(|_| (alloc(), alloc())).collect();

    pdf.catalog(catalog).pages(tree);
    pdf.pages(tree)
        .kids(pages.iter().map(|(page, _)| *page))
        .count(pages.len() as i32);
    let mut i = pdf.document_info(info);
    i.title(TextStr(&format!("{} – {}", doc.title, doc.subtitle)));
    i.creator(TextStr("Fixpunkt"));
    i.producer(TextStr("Fixpunkt"));
    i.finish();
    for (r, base) in [(regular, "Helvetica"), (bold, "Helvetica-Bold")] {
        pdf.type1_font(r)
            .base_font(Name(base.as_bytes()))
            .encoding_predefined(Name(b"WinAnsiEncoding"));
    }
    for (img, r) in p.images.iter().zip(&images) {
        let mut x = pdf.image_xobject(*r, &img.zlib);
        x.filter(Filter::FlateDecode);
        x.width(img.cols as i32);
        x.height(img.rows as i32);
        x.color_space().device_rgb();
        x.bits_per_component(8);
        // Karte nicht weichzeichnen: jede Zelle bleibt als Fläche erkennbar
        x.interpolate(false);
    }
    let names: Vec<String> = (0..images.len()).map(image_name).collect();
    for ((page, content), body) in pages.iter().zip(p.pages) {
        let mut pg = pdf.page(*page);
        pg.media_box(Rect::new(0.0, 0.0, PAGE_W, PAGE_H));
        pg.parent(tree);
        pg.contents(*content);
        let mut res = pg.resources();
        res.fonts()
            .pair(Name(b"F1"), regular)
            .pair(Name(b"F2"), bold);
        let mut xo = res.x_objects();
        for (n, r) in names.iter().zip(&images) {
            xo.pair(Name(n.as_bytes()), *r);
        }
        xo.finish();
        res.finish();
        pg.finish();
        pdf.stream(*content, &body.finish());
    }
    pdf.finish()
}

#[cfg(test)]
mod tests {
    use super::super::doc::{col, Kpi};
    use super::*;

    fn sample(rows: usize, map: Option<Map>) -> Doc {
        let mut blocks = vec![
            Block::Note("In mindestens einem Bereich fehlen Daten.".into()),
            Block::Heading("Massen je Bereich".into()),
            Block::Table(Table {
                columns: vec![col("Bereich", false, 2.0), col("Abtrag", true, 1.0)],
                rows: (0..rows)
                    .map(|i| vec![format!("Böschung {i}"), "800,1 m³".into()])
                    .collect(),
                total: Some(vec!["Summe".into(), "1.234,5 m³".into()]),
            }),
            Block::Text("Rastermethode, Zellgröße 25,0 cm.".into()),
        ];
        blocks.extend(map.map(Block::Map));
        Doc {
            title: "Mengenbericht".into(),
            subtitle: "Urgelände → Aushub".into(),
            meta: vec![("Baustelle".into(), "Kirchstraße 12".into())],
            kpis: vec![Kpi::new("Abtrag", "800,1 m³".into(), Tone::Removed)],
            blocks,
        }
    }

    fn page_texts(bytes: &[u8]) -> Vec<String> {
        let pdf = lopdf::Document::load_mem(bytes).expect("gültiges PDF");
        let pages = pdf.get_pages();
        pages
            .keys()
            .map(|n| pdf.extract_text(&[*n]).expect("Text lesbar"))
            .collect()
    }

    #[test]
    fn one_page_report_is_valid_pdf_with_text() {
        let bytes = render(&sample(3, None));
        assert!(bytes.starts_with(b"%PDF-"));
        let pages = page_texts(&bytes);
        assert_eq!(pages.len(), 1);
        let t = &pages[0];
        for s in [
            "Mengenbericht",
            "Urgelände » Aushub",
            "Kirchstraße 12",
            "800,1 m³",
            "Böschung 2",
            "Summe",
            "Seite 1 von 1",
        ] {
            assert!(t.contains(s), "{s} fehlt in {t}");
        }
    }

    #[test]
    fn long_tables_break_and_repeat_the_header() {
        let pages = page_texts(&render(&sample(80, None)));
        assert_eq!(pages.len(), 3, "80 Zeilen à 18 pt passen auf drei Seiten");
        assert!(pages[1].contains("Bereich"), "Kopfzeile auf Seite 2");
        assert!(pages[2].contains("1.234,5 m³"));
        assert!(pages[2].contains("Seite 3 von 3"));
    }

    #[test]
    fn heading_moves_with_the_map() {
        let map = Map {
            cols: 100,
            rows: 100,
            rgb: vec![200; 100 * 100 * 3],
            width_m: 40.0,
            outlines: Vec::new(),
            legend: vec![[0, 0, 0], [255, 255, 255]],
            legend_labels: ["a".into(), "0".into(), "b".into()],
            caption: "Bildunterschrift".into(),
        };
        let mut doc = sample(30, None);
        doc.blocks.push(Block::Heading("Höhenänderung".into()));
        doc.blocks.push(Block::Map(map));
        let pages = page_texts(&render(&doc));
        assert_eq!(pages.len(), 2);
        assert!(
            !pages[0].contains("Höhenänderung"),
            "Überschrift nicht allein unten"
        );
        assert!(pages[1].contains("Höhenänderung") && pages[1].contains("Bildunterschrift"));
    }

    #[test]
    fn map_shrinks_into_the_rest_of_the_page() {
        let map = Map {
            cols: 100,
            rows: 100,
            rgb: vec![200; 100 * 100 * 3],
            width_m: 40.0,
            outlines: Vec::new(),
            legend: vec![[0, 0, 0], [255, 255, 255]],
            legend_labels: ["a".into(), "0".into(), "b".into()],
            caption: "Bildunterschrift".into(),
        };
        let mut doc = sample(2, None);
        doc.blocks.push(Block::Heading("Höhenänderung".into()));
        doc.blocks.push(Block::Map(map));
        let pages = page_texts(&render(&doc));
        assert_eq!(pages.len(), 1, "Karte passt verkleinert auf Seite 1");
    }

    #[test]
    fn map_is_embedded_as_image() {
        let map = Map {
            cols: 4,
            rows: 2,
            rgb: vec![200; 4 * 2 * 3],
            width_m: 40.0,
            outlines: vec![("Grube".into(), vec![[0.1, 0.1], [0.9, 0.1], [0.9, 0.9]])],
            legend: vec![[29, 78, 216], [245, 245, 245], [180, 83, 9]],
            legend_labels: ["Auftrag 5 cm".into(), "0".into(), "Abtrag 5 cm".into()],
            caption: "Höhenänderung nachher − vorher.".into(),
        };
        let bytes = render(&sample(1, Some(map)));
        let pdf = lopdf::Document::load_mem(&bytes).unwrap();
        let images = pdf
            .objects
            .values()
            .filter_map(|o| o.as_stream().ok())
            .filter(|s| s.dict.get(b"Subtype").and_then(|v| v.as_name()).ok() == Some(b"Image"))
            .map(|s| {
                (
                    s.dict.get(b"Width").unwrap().as_i64().unwrap(),
                    s.decompressed_content().unwrap().len(),
                )
            })
            .collect::<Vec<_>>();
        // Karte 4 × 2 und Legende 3 × 1, je 3 Byte RGB
        assert!(images.contains(&(4, 24)), "{images:?}");
        assert!(images.contains(&(3, 9)), "{images:?}");
        let t = page_texts(&bytes).join(" ");
        assert!(
            t.contains("Grube") && t.contains("Abtrag 5 cm") && t.contains("5 m"),
            "{t}"
        );
    }

    #[test]
    fn scale_bar_is_round() {
        // 60 m breites Bild auf 500 pt: ein Fünftel sind 12 m → 10 m, ein Sechstel der Breite
        let (m, len) = scale_bar(60.0, 500.0);
        assert_eq!(m, 10.0);
        assert!((len - 500.0 / 6.0).abs() < 1e-3);
        assert_eq!(scale_bar(4.0, 100.0).0, 0.5);
        assert_eq!(scale_bar(1300.0, 100.0).0, 200.0);
    }
}
