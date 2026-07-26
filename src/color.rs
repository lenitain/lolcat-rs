#[derive(Clone, Copy)]
pub enum InterpolationMode {
    Linear,
    Oklch,
    Cubic,
}

#[derive(Clone, Copy)]
pub struct ColorStop {
    pub position: f64,
    pub color: (u8, u8, u8),
}

pub struct ColorGenerator {
    pub freq: f64,
    pub spread: f64,
    pub seed: f64,
    pub line_idx: usize,
    pub gradient: Vec<ColorStop>,
    pub smoothness: f64,
    pub interpolate: InterpolationMode,
    pub custom_gradient: bool,
}

impl ColorGenerator {
    pub fn next_line(&mut self) {
        self.line_idx += 1;
    }

    pub fn get_rgb(&self, char_idx: usize) -> (u8, u8, u8) {
        let i = self.seed + self.line_idx as f64 + char_idx as f64 / self.spread;

        // Use original sine wave algorithm for default rainbow
        if !self.custom_gradient {
            let r = ((self.freq * i).sin() * 150.0 + 128.0).clamp(0.0, 255.0) as u8;
            let g = ((self.freq * i + 2.0 * std::f64::consts::PI / 3.0).sin() * 150.0 + 128.0)
                .clamp(0.0, 255.0) as u8;
            let b = ((self.freq * i + 4.0 * std::f64::consts::PI / 3.0).sin() * 150.0 + 128.0)
                .clamp(0.0, 255.0) as u8;
            return (r, g, b);
        }

        let position = (self.freq * i).sin() * 0.5 + 0.5;
        let position = position.rem_euclid(1.0);

        self.interpolate_gradient(position)
    }

    fn interpolate_gradient(&self, position: f64) -> (u8, u8, u8) {
        if self.gradient.is_empty() {
            return (0, 0, 0);
        }

        if self.gradient.len() == 1 {
            return self.gradient[0].color;
        }

        let mut sorted = self.gradient.clone();
        sorted.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());

        if position <= sorted[0].position {
            return sorted[0].color;
        }

        let last = sorted.last().unwrap();
        if position >= last.position {
            let range = 1.0 - last.position;
            let t = if range > 0.0 {
                (position - last.position) / range
            } else {
                0.0
            };
            let t = apply_smoothness(t, self.smoothness);
            return self.interpolate_colors(last.color, sorted[0].color, t);
        }

        let (left, right) = find_surrounding_stops(&sorted, position);
        let t = (position - left.position) / (right.position - left.position);
        let t = apply_smoothness(t, self.smoothness);

        self.interpolate_colors(left.color, right.color, t)
    }

    fn interpolate_colors(&self, c1: (u8, u8, u8), c2: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
        match self.interpolate {
            InterpolationMode::Linear => interpolate_linear(c1, c2, t),
            InterpolationMode::Oklch => interpolate_oklch(c1, c2, t),
            InterpolationMode::Cubic => interpolate_cubic(c1, c2, t),
        }
    }

    pub fn write_colored_char<W: std::io::Write>(
        &self,
        w: &mut W,
        c: char,
        char_idx: usize,
    ) -> std::io::Result<()> {
        let (r, g, b) = self.get_rgb(char_idx);
        write!(w, "\x1b[38;2;{};{};{}m{}", r, g, b, c)
    }
}

fn find_surrounding_stops(sorted: &[ColorStop], position: f64) -> (&ColorStop, &ColorStop) {
    for i in 0..sorted.len() - 1 {
        if position >= sorted[i].position && position <= sorted[i + 1].position {
            return (&sorted[i], &sorted[i + 1]);
        }
    }
    (&sorted[0], &sorted[sorted.len() - 1])
}

fn apply_smoothness(t: f64, smoothness: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    if smoothness <= 0.0 {
        return t;
    }
    let s = smoothness.clamp(0.0, 1.0);
    let factor = 1.0 - s;
    let curve = t * t * (3.0 - 2.0 * t);
    t * (1.0 - factor) + curve * factor
}

fn interpolate_linear(c1: (u8, u8, u8), c2: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    let r = c1.0 as f64 + (c2.0 as f64 - c1.0 as f64) * t;
    let g = c1.1 as f64 + (c2.1 as f64 - c1.1 as f64) * t;
    let b = c1.2 as f64 + (c2.2 as f64 - c1.2 as f64) * t;
    (r as u8, g as u8, b as u8)
}

fn interpolate_cubic(c1: (u8, u8, u8), c2: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    let t2 = t * t;
    let t3 = t2 * t;
    let tt = 3.0 * t2 - 2.0 * t3;
    let r = c1.0 as f64 + (c2.0 as f64 - c1.0 as f64) * tt;
    let g = c1.1 as f64 + (c2.1 as f64 - c1.1 as f64) * tt;
    let b = c1.2 as f64 + (c2.2 as f64 - c1.2 as f64) * tt;
    (r as u8, g as u8, b as u8)
}

fn rgb_to_oklab(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;

    let r_lin = if r > 0.04045 {
        ((r + 0.055) / 1.055).powf(2.4)
    } else {
        r / 12.92
    };
    let g_lin = if g > 0.04045 {
        ((g + 0.055) / 1.055).powf(2.4)
    } else {
        g / 12.92
    };
    let b_lin = if b > 0.04045 {
        ((b + 0.055) / 1.055).powf(2.4)
    } else {
        b / 12.92
    };

    let l = 0.4122214708 * r_lin + 0.5363325363 * g_lin + 0.0514459929 * b_lin;
    let m = 0.2119034982 * r_lin + 0.6806995451 * g_lin + 0.1073969566 * b_lin;
    let s = 0.0883024619 * r_lin + 0.2817188376 * g_lin + 0.6299787005 * b_lin;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    let ll = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    let aa = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    let bb = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;

    (ll, aa, bb)
}

fn oklab_to_rgb(ll: f64, aa: f64, bb: f64) -> (u8, u8, u8) {
    let l_ = ll + 0.3963377774 * aa + 0.2158037573 * bb;
    let m_ = ll - 0.1055613458 * aa - 0.0638541728 * bb;
    let s_ = ll - 0.0894841775 * aa - 1.2914855480 * bb;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let b = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

    let r = r.clamp(0.0, 1.0);
    let g = g.clamp(0.0, 1.0);
    let b = b.clamp(0.0, 1.0);

    let r = if r > 0.0031308 {
        1.055 * r.powf(1.0 / 2.4) - 0.055
    } else {
        r * 12.92
    };
    let g = if g > 0.0031308 {
        1.055 * g.powf(1.0 / 2.4) - 0.055
    } else {
        g * 12.92
    };
    let b = if b > 0.0031308 {
        1.055 * b.powf(1.0 / 2.4) - 0.055
    } else {
        b * 12.92
    };

    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

fn interpolate_oklch(c1: (u8, u8, u8), c2: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let (l1, a1, b1) = rgb_to_oklab(c1.0, c1.1, c1.2);
    let (l2, a2, b2) = rgb_to_oklab(c2.0, c2.1, c2.2);

    let l = l1 + (l2 - l1) * t;
    let a = a1 + (a2 - a1) * t;
    let b = b1 + (b2 - b1) * t;

    oklab_to_rgb(l, a, b)
}

pub fn parse_gradient(spec: &str) -> Result<Vec<ColorStop>, String> {
    let mut stops = Vec::new();

    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let colon_pos = part
            .find(':')
            .ok_or_else(|| format!("Invalid format: {}", part))?;
        let position: f64 = part[..colon_pos]
            .parse()
            .map_err(|_| format!("Invalid position: {}", &part[..colon_pos]))?;

        let color_str = &part[colon_pos + 1..];
        let color = parse_color(color_str)?;

        stops.push(ColorStop {
            position: position / 100.0,
            color,
        });
    }

    if stops.is_empty() {
        return Err("No color stops provided".to_string());
    }

    Ok(stops)
}

pub fn parse_color(s: &str) -> Result<(u8, u8, u8), String> {
    let s = s.trim().trim_start_matches('#');

    let named = match s.to_lowercase().as_str() {
        "red" => Some((255, 0, 0)),
        "green" => Some((0, 255, 0)),
        "blue" => Some((0, 0, 255)),
        "yellow" => Some((255, 255, 0)),
        "cyan" => Some((0, 255, 255)),
        "magenta" => Some((255, 0, 255)),
        "white" => Some((255, 255, 255)),
        "black" => Some((0, 0, 0)),
        _ => None,
    };

    if let Some(c) = named {
        return Ok(c);
    }

    if s.len() != 6 {
        return Err(format!("Invalid hex color: {}", s));
    }

    let r = u8::from_str_radix(&s[0..2], 16).map_err(|_| format!("Invalid hex: {}", s))?;
    let g = u8::from_str_radix(&s[2..4], 16).map_err(|_| format!("Invalid hex: {}", s))?;
    let b = u8::from_str_radix(&s[4..6], 16).map_err(|_| format!("Invalid hex: {}", s))?;

    Ok((r, g, b))
}

pub fn default_rainbow() -> Vec<ColorStop> {
    // Placeholder - actual colors computed by sine wave in get_rgb()
    vec![ColorStop {
        position: 0.0,
        color: (0, 0, 0),
    }]
}
