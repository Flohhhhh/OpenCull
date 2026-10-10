//! Which faces a photograph is about, and which are in the background.
//!
//! The Mac app's rules, carried over line for line, with one change forced
//! by having no Apple parts — see `Signals::likely_subject`.

use crate::faces::FaceBox;

/// What the rules are told about one face.
#[derive(Clone, Copy, Debug)]
pub struct Signals {
    pub place: FaceBox,
    /// The capture-quality number, if there is one.
    pub quality: Option<f64>,
    /// How likely the judge thinks it is a subject, 0 to 1.
    ///
    /// Stands in for something the Mac app had and this app does not: where
    /// Apple's saliency said the eye is drawn. The Mac app would not call the
    /// largest face a poster if the eye was drawn to it. Here it is not
    /// called one if the judge thinks it a subject.
    pub likely_subject: f64,
}

impl Signals {
    fn linear(&self) -> f64 {
        (self.place.w * self.place.h).sqrt()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    OnlyFace,
    LargeInFrame,
    SameScale,
    BesideASubject,
    SmallAndIsolated,
    Poster,
    RescuedLargest,
    LowQuality,
}

impl Reason {
    /// What to say about a face left out. `None` for one kept.
    pub fn badge(self) -> Option<&'static str> {
        match self {
            Reason::Poster => Some("Background — reads as a print or sign"),
            Reason::SmallAndIsolated => Some("Background — small and away from the group"),
            Reason::LowQuality => Some("Background — face too indistinct to be a subject"),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decision {
    pub subject: bool,
    pub reason: Reason,
}

/// A face this indistinct is not who the photograph is about. MEASURED by
/// the Mac app against 188 hand-judged faces: at 0.15, 58% of background
/// removed for 1.3% of real subjects lost. Not to be raised above 0.20.
const MIN_QUALITY: f64 = 0.15;
/// Kept if at least this tall, as a share of the frame.
const KEEP_HEIGHT: f64 = 0.06;
/// Kept if at least this size against the largest face — LINEAR size. By
/// area, a child beside a parent is 0.42 and falls out; linearly, 0.65.
const KEEP_RELATIVE: f64 = 0.48;
/// Kept if within this many of its own widths of a face already kept.
const BESIDE: f64 = 3.0;
const POSTER_MIN_FACES: usize = 4;
const POSTER_SIZE: f64 = 2.0;
const POSTER_QUALITY: f64 = 0.6;

/// Decides, for the faces of one photograph together.
pub fn classify(faces: &[Signals]) -> Vec<Decision> {
    let keep = |reason| Decision { subject: true, reason };
    let drop = |reason| Decision { subject: false, reason };
    match faces.len() {
        0 => return Vec::new(),
        1 => return vec![keep(Reason::OnlyFace)],
        _ => {}
    }

    let linear: Vec<f64> = faces.iter().map(Signals::linear).collect();
    let mut sorted = linear.clone();
    sorted.sort_by(f64::total_cmp);
    let (largest, second) = (sorted[sorted.len() - 1], sorted[sorted.len() - 2]);
    let mut qualities: Vec<f64> = faces.iter().filter_map(|f| f.quality).collect();
    qualities.sort_by(f64::total_cmp);
    let median_quality = (qualities.len() >= 2).then(|| qualities[qualities.len() / 2]);
    let poster_possible = faces.len() >= POSTER_MIN_FACES && largest > second * POSTER_SIZE;

    let mut decisions: Vec<Decision> = faces
        .iter()
        .zip(&linear)
        .map(|(face, size)| {
            // One face far larger than the rest and far less distinct is a
            // print or a sign: a banner behind a table of guests.
            if poster_possible && *size >= largest * 0.98 {
                let reads_as_print = match (face.quality, median_quality) {
                    (Some(q), Some(median)) if median > 0.0 => q < median * POSTER_QUALITY,
                    (None, Some(_)) => true,
                    _ => false,
                };
                if reads_as_print && face.likely_subject < 0.5 {
                    return drop(Reason::Poster);
                }
            }
            if face.place.h >= KEEP_HEIGHT {
                keep(Reason::LargeInFrame)
            } else if largest > 0.0 && size / largest >= KEEP_RELATIVE {
                keep(Reason::SameScale)
            } else {
                drop(Reason::SmallAndIsolated)
            }
        })
        .collect();

    // A small face right beside a kept one belongs with it: the child at a
    // parent's shoulder. One step only, from the faces kept above.
    let seeds: Vec<usize> = (0..faces.len()).filter(|i| decisions[*i].subject).collect();
    for i in 0..faces.len() {
        if decisions[i].subject || decisions[i].reason == Reason::Poster {
            continue;
        }
        let own = faces[i].place.w.max(0.001);
        let beside = seeds.iter().any(|s| {
            let (dx, dy) = (faces[*s].place.mid_x - faces[i].place.mid_x, faces[*s].place.mid_y - faces[i].place.mid_y);
            (dx * dx + dy * dy).sqrt() / own <= BESIDE
        });
        if beside {
            decisions[i] = keep(Reason::BesideASubject);
        }
    }

    // After the step above, which was reaching past a real subject into the
    // crowd. A missing reading is not a low one, and is let through.
    for (decision, face) in decisions.iter_mut().zip(faces) {
        if decision.subject && face.quality.is_some_and(|q| q < MIN_QUALITY) {
            *decision = drop(Reason::LowQuality);
        }
    }

    // A photograph with faces in it is never left with no subject.
    if !decisions.iter().any(|d| d.subject) {
        let biggest = (0..faces.len()).max_by(|a, b| linear[*a].total_cmp(&linear[*b])).expect("at least two faces");
        decisions[biggest] = keep(Reason::RescuedLargest);
    }
    decisions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face(x: f64, y: f64, size: f64, quality: f64) -> Signals {
        Signals { place: FaceBox { mid_x: x, mid_y: y, w: size, h: size }, quality: Some(quality), likely_subject: 0.9 }
    }

    fn verdicts(faces: &[Signals]) -> Vec<(bool, Reason)> {
        classify(faces).into_iter().map(|d| (d.subject, d.reason)).collect()
    }

    #[test]
    fn one_face_is_the_subject_whatever_it_looks_like() {
        assert_eq!(verdicts(&[face(0.5, 0.5, 0.01, 0.01)]), [(true, Reason::OnlyFace)]);
        assert!(classify(&[]).is_empty());
    }

    #[test]
    fn a_child_beside_a_parent_is_kept_by_linear_size() {
        // 0.65 of the parent linearly; 0.42 by area, which would have lost them.
        let got = verdicts(&[face(0.4, 0.5, 0.05, 0.5), face(0.46, 0.45, 0.0325, 0.5)]);
        assert_eq!(got[1], (true, Reason::SameScale));
    }

    #[test]
    fn a_small_face_far_from_anyone_is_background_and_one_at_a_shoulder_is_not() {
        let big = face(0.3, 0.5, 0.05, 0.5);
        let far = face(0.9, 0.5, 0.01, 0.5);
        let near = face(0.325, 0.5, 0.01, 0.5);
        let got = verdicts(&[big, far, near]);
        assert_eq!(got[1], (false, Reason::SmallAndIsolated));
        assert_eq!(got[2], (true, Reason::BesideASubject));
    }

    #[test]
    fn an_indistinct_face_is_background_even_beside_a_subject() {
        let got = verdicts(&[face(0.3, 0.5, 0.05, 0.5), face(0.325, 0.5, 0.01, 0.05), face(0.6, 0.5, 0.08, 0.10)]);
        assert_eq!(got[1], (false, Reason::LowQuality), "reached by the shoulder rule, then put back");
        assert_eq!(got[2], (false, Reason::LowQuality), "large in the frame is not enough either");
    }

    #[test]
    fn a_face_with_no_quality_reading_is_not_treated_as_a_low_one() {
        let mut unread = face(0.6, 0.5, 0.08, 0.0);
        unread.quality = None;
        assert_eq!(verdicts(&[face(0.3, 0.5, 0.05, 0.5), unread])[1], (true, Reason::LargeInFrame));
    }

    #[test]
    fn a_photo_is_never_left_with_no_subject() {
        let got = verdicts(&[face(0.3, 0.5, 0.05, 0.05), face(0.6, 0.5, 0.07, 0.05)]);
        assert_eq!(got, [(false, Reason::LowQuality), (true, Reason::RescuedLargest)]);
    }

    #[test]
    fn a_huge_indistinct_face_behind_a_group_is_a_poster_unless_the_judge_says_subject() {
        let guests = [face(0.2, 0.3, 0.07, 0.5), face(0.4, 0.3, 0.07, 0.5), face(0.6, 0.3, 0.07, 0.5)];
        let mut banner = face(0.5, 0.7, 0.2, 0.2);
        banner.likely_subject = 0.1;
        let mut faces = guests.to_vec();
        faces.push(banner);
        assert_eq!(verdicts(&faces)[3], (false, Reason::Poster));
        assert!(verdicts(&faces)[..3].iter().all(|v| v.0));

        // The same picture, but the judge takes the large face for a person
        // the photo is about: a close-up in front of guests, slightly soft.
        faces[3].likely_subject = 0.9;
        assert_eq!(verdicts(&faces)[3], (true, Reason::LargeInFrame));
    }
}
