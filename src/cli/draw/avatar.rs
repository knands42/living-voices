use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, StatefulWidget, Widget};
use ratatui_image::StatefulImage;
use ratatui_image::protocol::StatefulProtocol;

use super::theme::{INK, speaker_color};

/// Anything that can paint itself into a rectangle. Pages talk to this.
pub trait Avatar {
    fn render(&mut self, area: Rect, buf: &mut Buffer);
}

// ---------- GlyphAvatar: the always-works default ----------

#[derive(Clone, Copy, Debug)]
pub struct GlyphAvatar {
    initial: char,
    tint: Color,
}

impl GlyphAvatar {
    pub fn from_name(name: &str) -> Self {
        let initial = name
            .chars()
            .find(|c| c.is_alphanumeric())
            .map(|c| c.to_ascii_uppercase())
            .unwrap_or('?');
        Self {
            initial,
            tint: speaker_color(name),
        }
    }
}

impl Avatar for GlyphAvatar {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        // Paint a solid tinted field with top/bottom half-blocks so even
        // half-cell heights read as a square. Walking the buffer directly
        // avoids building a Paragraph just for the fill.
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)]
                    .set_char('█')
                    .set_fg(self.tint)
                    .set_bg(Color::Reset);
            }
        }

        // Draw the initial centered, in high-contrast ink.
        let letter = Paragraph::new(Line::from(Span::styled(
            self.initial.to_string(),
            Style::default()
                .fg(INK)
                .bg(self.tint)
                .add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center);

        // Vertically center by shrinking a one-line slice at mid-height.
        let mid = area.top() + area.height / 2;
        let letter_area = Rect {
            x: area.left(),
            y: mid,
            width: area.width,
            height: 1,
        };
        letter.render(letter_area, buf);
    }
}

// ---------- ImageAvatar: wire a picture when the terminal supports it ----------

#[allow(dead_code)] // seam: wired by callers once character portraits exist
pub struct ImageAvatar {
    protocol: StatefulProtocol,
}

impl ImageAvatar {
    #[allow(dead_code)]
    pub fn new(protocol: StatefulProtocol) -> Self {
        Self { protocol }
    }
}

impl Avatar for ImageAvatar {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        StatefulImage::default().render(area, buf, &mut self.protocol);
    }
}
