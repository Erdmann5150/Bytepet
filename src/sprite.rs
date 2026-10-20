//! Monochrome sprites parsed from text art ('#' = dot).

#[derive(Clone, Debug, PartialEq)]
pub struct Sprite {
    pub w: i32,
    pub h: i32,
    /// Lit dots, row-major.
    pub pix: Vec<bool>,
    /// Filled silhouette (dots plus enclosed interior), used to blank what's behind a sprite.
    pub mask: Vec<bool>,
}

impl Sprite {
    pub fn parse(text: &str) -> Sprite {
        let rows: Vec<&str> = text.lines().map(|l| l.trim_end()).filter(|l| !l.trim().is_empty()).collect();
        Sprite::from_rows(&rows)
    }

    pub fn from_rows<S: AsRef<str>>(rows: &[S]) -> Sprite {
        let h = rows.len() as i32;
        let w = rows.iter().map(|r| r.as_ref().chars().count()).max().unwrap_or(0) as i32;
        let mut pix = vec![false; (w * h) as usize];
        for (y, r) in rows.iter().enumerate() {
            for (x, c) in r.as_ref().chars().enumerate() {
                if c == '#' {
                    pix[y * w as usize + x] = true;
                }
            }
        }
        let mask = solid_mask(w, h, &pix);
        Sprite { w, h, pix, mask }
    }

    pub fn get(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h && self.pix[(y * self.w + x) as usize]
    }

    pub fn rows(&self) -> Vec<String> {
        (0..self.h).map(|y| (0..self.w).map(|x| if self.get(x, y) { '#' } else { '.' }).collect()).collect()
    }
}

/// Everything not reachable from the border through empty cells.
fn solid_mask(w: i32, h: i32, pix: &[bool]) -> Vec<bool> {
    let mut outside = vec![false; (w * h) as usize];
    let mut stack: Vec<(i32, i32)> = Vec::new();
    for x in 0..w {
        stack.push((x, 0));
        stack.push((x, h - 1));
    }
    for y in 0..h {
        stack.push((0, y));
        stack.push((w - 1, y));
    }
    while let Some((x, y)) = stack.pop() {
        if x < 0 || y < 0 || x >= w || y >= h {
            continue;
        }
        let i = (y * w + x) as usize;
        if outside[i] || pix[i] {
            continue;
        }
        outside[i] = true;
        stack.extend_from_slice(&[(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]);
    }
    outside.iter().map(|o| !o).collect()
}
