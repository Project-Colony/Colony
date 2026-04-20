//! Augmented Markdown renderer for the detail view.
//!
//! Pre-processes a raw Markdown string into a sequence of `DetailBlock`s so
//! two constructs can be rendered with native Colony widgets instead of the
//! default iced markdown output:
//!
//! - **shields.io badges** — `[![alt](img.shields.io/badge/..)](link)` lines
//!   become colored pills matching our palette (no network round-trip).
//! - **GFM pipe tables** — parsed line-based and rendered via iced's `table`
//!   widget directly, skipping iced markdown's internal horizontal
//!   `scrollable` that captures vertical wheel events.
//!
//! Everything else flows through iced's `markdown::view_with` using a
//! caller-supplied `Viewer`.
use iced::font::Weight;
use iced::widget::{column, container, markdown, row, text, table as tbl, Row};
use iced::{alignment, Color, Element, Length};

use crate::message::Message;
use crate::ui::theme::Palette;

#[derive(Debug, Clone)]
pub enum DetailBlock {
    Markdown(Vec<markdown::Item>),
    Badges(Vec<Badge>),
    Table(TableData),
}

#[derive(Debug, Clone)]
pub struct Badge {
    pub label: String,
    pub value: String,
    pub color_hex: [u8; 3],
    pub link: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// Parse a Markdown document into a block sequence. Badge-only paragraphs
/// and pipe-tables are peeled off; the remaining lines are re-joined and
/// handed to iced's `markdown::parse`.
pub fn parse(md: &str) -> Vec<DetailBlock> {
    let lines: Vec<&str> = md.lines().collect();
    let mut blocks: Vec<DetailBlock> = Vec::new();
    let mut md_buffer: Vec<&str> = Vec::new();
    let flush_md = |buf: &mut Vec<&str>, out: &mut Vec<DetailBlock>| {
        if buf.is_empty() {
            return;
        }
        let joined = buf.join("\n");
        let items: Vec<markdown::Item> = markdown::parse(&joined).collect();
        out.push(DetailBlock::Markdown(items));
        buf.clear();
    };

    let mut i = 0;
    while i < lines.len() {
        // Pipe table?
        if let Some((table, consumed)) = try_parse_table(&lines[i..]) {
            flush_md(&mut md_buffer, &mut blocks);
            blocks.push(DetailBlock::Table(table));
            i += consumed;
            continue;
        }
        // Badge-only run?
        if let Some((badges, consumed)) = try_parse_badges(&lines[i..]) {
            flush_md(&mut md_buffer, &mut blocks);
            blocks.push(DetailBlock::Badges(badges));
            i += consumed;
            continue;
        }
        md_buffer.push(lines[i]);
        i += 1;
    }
    flush_md(&mut md_buffer, &mut blocks);
    blocks
}

fn try_parse_table(lines: &[&str]) -> Option<(TableData, usize)> {
    if lines.len() < 2 {
        return None;
    }
    let header = lines[0].trim();
    let sep = lines[1].trim();
    if !is_pipe_row(header) || !is_pipe_separator(sep) {
        return None;
    }
    let headers = split_pipe_row(header);
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut consumed = 2;
    while consumed < lines.len() {
        let line = lines[consumed].trim();
        if !is_pipe_row(line) {
            break;
        }
        rows.push(split_pipe_row(line));
        consumed += 1;
    }
    Some((TableData { headers, rows }, consumed))
}

fn is_pipe_row(line: &str) -> bool {
    line.starts_with('|') && line.ends_with('|') && line.matches('|').count() >= 2
}

fn is_pipe_separator(line: &str) -> bool {
    line.starts_with('|')
        && line.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
        && line.contains('-')
}

fn split_pipe_row(line: &str) -> Vec<String> {
    let trimmed = line.trim_start_matches('|').trim_end_matches('|');
    trimmed
        .split('|')
        .map(|s| s.trim().to_string())
        .collect()
}

/// Detect a contiguous block of lines that are each exclusively a
/// shields.io badge link. Returns the parsed badges and how many lines
/// were consumed.
fn try_parse_badges(lines: &[&str]) -> Option<(Vec<Badge>, usize)> {
    let mut badges = Vec::new();
    let mut consumed = 0;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if consumed == 0 {
                return None;
            }
            consumed += 1;
            continue;
        }
        match parse_badge_line(trimmed) {
            Some(badge) => {
                badges.push(badge);
                consumed += 1;
            }
            None => break,
        }
    }
    if badges.is_empty() {
        None
    } else {
        Some((badges, consumed))
    }
}

/// Accept either `[![alt](img)](link)` or `![alt](img)` — returns a Badge
/// when the image URL is a shields.io `/badge/{label}-{value}-{color}`.
fn parse_badge_line(line: &str) -> Option<Badge> {
    // Find the `![` of the image.
    let (img_start, link): (usize, Option<String>) = if let Some(_stripped) =
        line.strip_prefix("[![")
    {
        // Wrapped form: [![alt](img)](link)
        // Locate the closing `)` of the image, then the opening `(` of link.
        let after = &line[3..]; // after [![
        // Find the first `](` which ends the alt text.
        let alt_end = after.find("](")?;
        let rest = &after[alt_end + 2..]; // after ](
        let img_end = find_matching_paren(rest)?;
        let img_url = &rest[..img_end];
        let after_img = &rest[img_end + 1..]; // after )
        let after_img = after_img.strip_prefix("](")?;
        let link_end = find_matching_paren(after_img)?;
        let link = after_img[..link_end].to_string();
        // Compute img_start for the unwrapped-image path (unused here, but keeps symmetry).
        let _ = img_start_placeholder();
        return parse_shields_url(img_url)
            .map(|(label, value, color)| Badge { label, value, color_hex: color, link: Some(link) });
    } else if line.starts_with("![") {
        (0, None)
    } else {
        return None;
    };
    let _ = img_start;
    // Unwrapped image form: ![alt](img)
    let after = line.strip_prefix("![")?;
    let alt_end = after.find("](")?;
    let rest = &after[alt_end + 2..];
    let img_end = find_matching_paren(rest)?;
    let img_url = &rest[..img_end];
    parse_shields_url(img_url).map(|(label, value, color)| Badge {
        label,
        value,
        color_hex: color,
        link,
    })
}

fn img_start_placeholder() -> usize { 0 }

fn find_matching_paren(s: &str) -> Option<usize> {
    let mut depth = 1i32;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Parse a shields.io /badge/{label}-{value}-{color} URL.
fn parse_shields_url(url: &str) -> Option<(String, String, [u8; 3])> {
    let path = url.split("shields.io/badge/").nth(1)?;
    // Drop query string and extension.
    let path = path.split('?').next().unwrap_or("");
    let path = path
        .strip_suffix(".svg")
        .or_else(|| path.strip_suffix(".png"))
        .unwrap_or(path);
    // shields.io encodes `-` inside label/value as `--`. Replace them with a
    // placeholder, split on single `-`, then restore.
    let marker = "\x00DASH\x00";
    let prepared = path.replace("--", marker);
    let parts: Vec<&str> = prepared.rsplitn(3, '-').collect();
    if parts.len() < 3 {
        return None;
    }
    let color_raw = parts[0];
    let value_raw = parts[1];
    let label_raw = parts[2];
    let decode = |s: &str| -> String {
        s.replace(marker, "-")
            .replace('_', " ")
            .split('%')
            .enumerate()
            .map(|(i, part)| {
                if i == 0 {
                    part.to_string()
                } else if part.len() >= 2 {
                    let hex = &part[..2];
                    let byte = u8::from_str_radix(hex, 16).unwrap_or(b'?');
                    format!("{}{}", byte as char, &part[2..])
                } else {
                    format!("%{}", part)
                }
            })
            .collect::<String>()
    };
    let color = badge_color_rgb(color_raw);
    Some((decode(label_raw), decode(value_raw), color))
}

/// Approximate shields.io color names → RGB. Unknown names fall back to
/// a neutral grey.
fn badge_color_rgb(name: &str) -> [u8; 3] {
    // Accept hex as `#rrggbb` or `rrggbb`.
    if let Some(h) = name.strip_prefix('#').or(Some(name)) {
        if h.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&h[0..2], 16),
                u8::from_str_radix(&h[2..4], 16),
                u8::from_str_radix(&h[4..6], 16),
            ) {
                return [r, g, b];
            }
        }
    }
    match name.to_ascii_lowercase().as_str() {
        "brightgreen" => [0x4c, 0xc1, 0x40],
        "green" => [0x97, 0xca, 0x00],
        "yellowgreen" => [0xa4, 0xa6, 0x1d],
        "yellow" => [0xdf, 0xb3, 0x17],
        "orange" => [0xfe, 0x7d, 0x37],
        "red" => [0xe0, 0x5d, 0x44],
        "blue" => [0x00, 0x7e, 0xc6],
        "lightgrey" | "lightgray" => [0x9f, 0x9f, 0x9f],
        "grey" | "gray" => [0x55, 0x55, 0x55],
        "purple" => [0x94, 0x3a, 0xa0],
        "pink" => [0xe7, 0x30, 0xa8],
        "black" => [0x1f, 0x1f, 0x1f],
        "white" => [0xee, 0xee, 0xee],
        _ => [0x80, 0x80, 0x80],
    }
}

// --- Rendering ---

pub fn view<'a, V>(
    blocks: &'a [DetailBlock],
    settings: markdown::Settings,
    viewer: &'a V,
) -> Element<'a, Message>
where
    V: markdown::Viewer<'a, Message> + 'a,
{
    let elements: Vec<Element<'a, Message>> = blocks
        .iter()
        .map(|block| match block {
            DetailBlock::Markdown(items) => markdown::view_with(items, settings, viewer),
            DetailBlock::Badges(badges) => view_badges(badges),
            DetailBlock::Table(data) => view_table(data),
        })
        .collect();
    column(elements).spacing(12).into()
}

fn view_badges(badges: &[Badge]) -> Element<'_, Message> {
    let children: Vec<Element<'_, Message>> = badges
        .iter()
        .map(badge_pill)
        .collect();
    Row::with_children(children)
        .spacing(6)
        .wrap()
        .into()
}

fn badge_pill(b: &Badge) -> Element<'_, Message> {
    let color = Color::from_rgb8(b.color_hex[0], b.color_hex[1], b.color_hex[2]);
    let label_text = text(b.label.clone())
        .size(12)
        .color(Palette::TEXT_PRIMARY());
    let value_text = text(b.value.clone()).size(12).color(Color::WHITE);

    let label_box = container(label_text)
        .padding([3, 8])
        .style(move |_theme| container::Style {
            background: Some(Palette::BG_CARD().into()),
            border: iced::Border {
                radius: iced::border::Radius::new(0)
                    .top_left(4)
                    .bottom_left(4),
                ..Default::default()
            },
            ..Default::default()
        });
    let value_box = container(value_text)
        .padding([3, 8])
        .style(move |_theme| container::Style {
            background: Some(color.into()),
            border: iced::Border {
                radius: iced::border::Radius::new(0)
                    .top_right(4)
                    .bottom_right(4),
                ..Default::default()
            },
            ..Default::default()
        });
    let pill: Element<'_, Message> =
        row![label_box, value_box].spacing(0).into();

    if let Some(link) = &b.link {
        let url = link.clone();
        iced::widget::button(pill)
            .on_press(Message::OpenUrl(url))
            .padding(0)
            .style(|_theme, status| iced::widget::button::Style {
                background: Some(iced::Color::TRANSPARENT.into()),
                text_color: Palette::TEXT_PRIMARY(),
                border: iced::Border {
                    radius: iced::border::Radius::new(4),
                    width: if matches!(status, iced::widget::button::Status::Hovered) {
                        1.0
                    } else {
                        0.0
                    },
                    color: Palette::ACCENT(),
                },
                ..Default::default()
            })
            .into()
    } else {
        pill
    }
}

fn view_table(data: &TableData) -> Element<'_, Message> {
    let headers = data.headers.clone();
    let rows: Vec<Vec<String>> = data.rows.clone();

    let columns = headers.iter().enumerate().map(|(i, header)| {
        let h_idx = i;
        tbl::column(
            text(header.clone())
                .size(13)
                .font(iced::Font {
                    weight: Weight::Bold,
                    ..iced::Font::default()
                })
                .color(Palette::TEXT_PRIMARY()),
            move |row: Vec<String>| -> Element<'_, Message> {
                let cell = row.get(h_idx).cloned().unwrap_or_default();
                text(cell)
                    .size(13)
                    .color(Palette::TEXT_SECONDARY())
                    .into()
            },
        )
        .align_x(alignment::Horizontal::Left)
        .width(Length::Shrink)
    });

    let table_widget = tbl(columns, rows)
        .padding_x(10.0)
        .padding_y(6.0)
        .separator_x(0);

    container(table_widget)
        .padding([8, 0])
        .width(Length::Shrink)
        .into()
}
