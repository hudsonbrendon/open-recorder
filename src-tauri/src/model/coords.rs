/// Converte um ponto global para coordenadas relativas à fonte (origem
/// superior-esquerda). `rect` = [x, y, w, h]. None se cair fora.
pub fn map_to_source(rect: [i64; 4], x: i64, y: i64) -> Option<(i64, i64)> {
    let [rx, ry, rw, rh] = rect;
    let rel_x = x - rx;
    let rel_y = y - ry;
    if rel_x < 0 || rel_y < 0 || rel_x > rw || rel_y > rh {
        return None;
    }
    Some((rel_x, rel_y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_inside_point_to_relative() {
        assert_eq!(map_to_source([100, 50, 800, 600], 150, 90), Some((50, 40)));
    }

    #[test]
    fn maps_top_left_to_zero() {
        assert_eq!(map_to_source([100, 50, 800, 600], 100, 50), Some((0, 0)));
    }

    #[test]
    fn returns_none_outside_left() {
        assert_eq!(map_to_source([100, 50, 800, 600], 99, 90), None);
    }

    #[test]
    fn returns_none_outside_bottom() {
        assert_eq!(map_to_source([100, 50, 800, 600], 150, 651), None);
    }
}
