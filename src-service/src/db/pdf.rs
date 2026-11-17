//! Minimal native PDF writer (no external crates).
//!
//! Produces a clean, valid, text-based PDF report: title, subtitle, and lines
//! (one per report row). Multi-page supported. ASCII-only (non-ASCII → '?').

const PAGE_W: f64 = 612.0; // US Letter
const PAGE_H: f64 = 792.0;
const MARGIN: f64 = 50.0;
const LINE_H: f64 = 15.0;
const FONT_SIZE: f64 = 11.0;

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            '→' => out.push_str("->"),
            '—' | '–' => out.push('-'),
            '·' => out.push('.'),
            '‘' | '’' => out.push('\''),
            '“' | '”' => out.push('"'),
            c if (c as u32) < 256 => out.push(c), // WinAnsi / Latin-1 safe
            _ => out.push('?'),
        }
    }
    out
}

fn wrap(line: &str, max_chars: usize) -> Vec<String> {
    if line.chars().count() <= max_chars {
        return vec![line.to_string()];
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    for w in line.split(' ') {
        if cur.chars().count() + w.chars().count() + 1 > max_chars {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            cur = w.to_string();
        } else {
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(w);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn content_stream(ops: &[(f64, String)]) -> Vec<u8> {
    let mut s = String::new();
    s.push_str("BT\n");
    for (y, text) in ops {
        s.push_str(&format!("/F1 {} Tf\n", FONT_SIZE));
        // Absolute text-matrix positioning (Td is relative → would accumulate).
        s.push_str(&format!("1 0 0 1 {} {} Tm\n", MARGIN, y));
        s.push_str(&format!("({}) Tj\n", escape(text)));
    }
    s.push_str("ET\n");
    s.into_bytes()
}

/// Render a multi-page PDF. `lines` are one per report row (already final text).
pub fn render_pdf(title: &str, subtitle: &str, lines: &[String]) -> Vec<u8> {
    const MAX_WRAP: usize = 88;
    const LINES_PER_PAGE: usize = 44;

    // Build (y, text) operators per page.
    let mut pages: Vec<Vec<(f64, String)>> = Vec::new();
    let mut ops: Vec<(f64, String)> = Vec::new();
    let mut y = PAGE_H - MARGIN;

    ops.push((y, title.to_string()));
    y -= 20.0;
    ops.push((y, subtitle.to_string()));
    y -= 16.0;

    for line in lines {
        for part in wrap(line, MAX_WRAP) {
            if ops.len().saturating_sub(2) >= LINES_PER_PAGE {
                pages.push(std::mem::take(&mut ops));
                y = PAGE_H - MARGIN;
            }
            ops.push((y, part));
            y -= LINE_H;
        }
    }
    if !ops.is_empty() {
        pages.push(ops);
    }
    if pages.is_empty() {
        pages.push(vec![(PAGE_H - MARGIN, "(no data)".to_string())]);
    }

    let n_pages = pages.len();
    // Objects: 1 catalog, 2 pages, then 2 per page (page+contents) starting at 3.
    // So the shared font object is the next free number.
    let font_num = 3 + n_pages * 2;

    // Build page objects (page + contents), numbered 3..(3+2n).
    let mut page_objs: Vec<Vec<u8>> = Vec::new();
    for i in 0..n_pages {
        let page_num = 3 + i * 2;
        let contents_num = page_num + 1;
        let stream = content_stream(&pages[i]);
        let page = format!(
            "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Contents {} 0 R /Resources << /Font << /F1 {} 0 R >> >> >>\nendobj\n",
            page_num, PAGE_W, PAGE_H, contents_num, font_num
        );
        let mut cobj = format!("{} 0 obj\n<< /Length {} >>\nstream\n", contents_num, stream.len()).into_bytes();
        cobj.extend_from_slice(&stream);
        cobj.extend_from_slice(b"\nendstream\nendobj\n");
        page_objs.push(page.into_bytes());
        page_objs.push(cobj);
    }

    let mut kids = String::new();
    for i in 0..n_pages {
        kids.push_str(&format!("{} 0 R ", 3 + i * 2));
    }
    let pages_obj = format!(
        "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
        kids.trim(),
        n_pages
    );
    let font_obj = format!(
        "{} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
        font_num
    );

    // Serialize with correct numeric order: 1 catalog, 2 pages, 3.. pages, font.
    fn push_obj(out: &mut Vec<u8>, xref: &mut Vec<i64>, obj: &[u8]) {
        xref.push(out.len() as i64);
        out.extend_from_slice(obj);
    }

    let mut out: Vec<u8> = Vec::new();
    let mut xref: Vec<i64> = Vec::new();

    out.extend_from_slice(b"%PDF-1.4\n");
    push_obj(&mut out, &mut xref, b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    push_obj(&mut out, &mut xref, pages_obj.as_bytes());
    for p in &page_objs {
        push_obj(&mut out, &mut xref, p);
    }
    push_obj(&mut out, &mut xref, font_obj.as_bytes());

    let xref_start = out.len() as i64;
    out.extend_from_slice(format!("xref\n0 {}\n", xref.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &xref {
        out.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            xref.len() + 1,
            xref_start
        )
        .as_bytes(),
    );

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_valid_pdf() {
        let bytes = render_pdf(
            "Vigil report",
            "Generated test",
            &[
                "Row one".to_string(),
                "Row two with a very long label that should wrap gracefully".to_string(),
            ],
        );
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.starts_with("%PDF-1.4"));
        assert!(s.contains("/Type /Catalog"));
        assert!(s.contains("/Type /Page"));
        assert!(s.contains("startxref"));
        assert!(s.contains("%%EOF"));
        assert!(s.contains("Tj"));
    }
}
