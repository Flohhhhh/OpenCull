//! Which frames came from one press of the shutter.
//!
//! A burst is frames taken less than 0.45 seconds apart — the figure the Mac
//! app arrived at by measurement. The Mac app then lets the picture veto that:
//! frames close in time that look nothing alike are split. That second half
//! leans on Apple's image analysis and is not here; this is the clock alone.

/// Longest gap between two frames of the same burst, in milliseconds.
pub const BURST_WINDOW_MS: f64 = 450.0;

/// Groups frames into bursts by capture time.
///
/// `times` is each frame's time on the camera's clock, in the order the
/// frames are held; `None` for a frame with no readable time. The result
/// gives each frame the number of its burst, or `None` if it stands alone.
///
/// Frames are compared in time order, whatever order they arrive in, so the
/// answer does not change when the folder is sorted another way.
pub fn group(times: &[Option<f64>]) -> Vec<Option<u32>> {
    let mut order: Vec<(usize, f64)> =
        times.iter().enumerate().filter_map(|(i, t)| t.map(|t| (i, t))).collect();
    // Ties keep folder order, so two frames stamped the same instant stay in
    // the order the camera numbered them.
    order.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));

    let mut groups = vec![None; times.len()];
    let mut next_id = 0u32;
    let mut run: Vec<usize> = Vec::new();

    let mut close = |run: &mut Vec<usize>, groups: &mut Vec<Option<u32>>| {
        // One frame on its own is not a burst.
        if run.len() > 1 {
            for &i in run.iter() {
                groups[i] = Some(next_id);
            }
            next_id += 1;
        }
        run.clear();
    };

    let mut previous: Option<f64> = None;
    for (index, time) in order {
        if let Some(prev) = previous {
            if time - prev > BURST_WINDOW_MS {
                close(&mut run, &mut groups);
            }
        }
        run.push(index);
        previous = Some(time);
    }
    close(&mut run, &mut groups);
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    fn times(ms: &[f64]) -> Vec<Option<f64>> {
        ms.iter().map(|&t| Some(t)).collect()
    }

    #[test]
    fn frames_inside_the_window_are_one_burst() {
        let g = group(&times(&[0.0, 100.0, 200.0, 5000.0, 5100.0, 9000.0]));
        assert_eq!(g, vec![Some(0), Some(0), Some(0), Some(1), Some(1), None]);
    }

    #[test]
    fn the_window_is_between_neighbours_not_from_the_first_frame() {
        // Each gap is 400 ms. The last frame is 1.2 s after the first and is
        // still the same burst, because the shutter never let up.
        let g = group(&times(&[0.0, 400.0, 800.0, 1200.0]));
        assert_eq!(g, vec![Some(0); 4]);
    }

    #[test]
    fn a_gap_of_exactly_the_window_still_joins_and_just_over_does_not() {
        assert_eq!(group(&times(&[0.0, 450.0])), vec![Some(0), Some(0)]);
        assert_eq!(group(&times(&[0.0, 450.1])), vec![None, None]);
    }

    #[test]
    fn the_answer_does_not_depend_on_the_order_frames_are_held_in() {
        // The same four frames, newest first.
        let g = group(&times(&[5100.0, 5000.0, 100.0, 0.0]));
        assert_eq!(g[0], g[1]);
        assert_eq!(g[2], g[3]);
        assert_ne!(g[0], g[2]);
        assert!(g.iter().all(Option::is_some));
    }

    #[test]
    fn a_frame_with_no_time_stands_alone_and_does_not_split_a_burst() {
        let g = group(&[Some(0.0), None, Some(100.0)]);
        assert_eq!(g, vec![Some(0), None, Some(0)]);
    }

    #[test]
    fn nothing_in_nothing_out() {
        assert!(group(&[]).is_empty());
        assert_eq!(group(&[Some(1.0)]), vec![None]);
    }
}
