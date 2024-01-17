use super::{BracketPair, Class, Level, OpeningBracket};
use core::{
  cmp::{max, min},
  ops::{Index, IndexMut, Range},
};

#[derive(PartialEq)]
pub(crate) enum OverrideStatus {
  Neutral,
  RTL,
  LTR,
  Isolate,
}

struct Status {
  level: Level,
  status: OverrideStatus,
}

#[inline(always)]
fn sliced<T: Index<Range<usize>> + ?Sized>(
  slicable: &T,
  range: Range<usize>,
) -> &<T as Index<Range<usize>>>::Output {
  slicable.index(range)
}

#[inline(always)]
fn sliced_mut<T: IndexMut<Range<usize>> + ?Sized>(
  slicable: &mut T,
  range: Range<usize>,
) -> &mut <T as Index<Range<usize>>>::Output {
  slicable.index_mut(range)
}

pub(crate) struct IsolatingRunSequence {
  runs: Vec<Range<usize>>,
  start_class: Class,
  end_class: Class,
}

impl IsolatingRunSequence {
  pub(crate) fn resolve_implicit_weak(&self, text: &str, processing_classes: &mut [Class]) {
    let mut prev_class_before_w4 = self.start_class;
    let mut prev_class_before_w5 = self.start_class;
    let mut prev_class_before_w1 = self.start_class;

    let mut last_strong_is_al = false;

    let mut et_run_indices = Vec::new();
    let mut bn_run_indices = Vec::new();

    for (run_index, level_run) in self.runs.iter().enumerate() {
      for i in &mut level_run.clone() {
        if processing_classes[i] == Class::BN {
          bn_run_indices.push(i);
          continue;
        }

        let mut w2_processing_class = processing_classes[i];
        prev_class_before_w1 = processing_classes[i];

        match processing_classes[i] {
          Class::EN => {
            if last_strong_is_al {
              processing_classes[i] = Class::AN;
            }
          }
          Class::AL => processing_classes[i] = Class::R,
          _ => {}
        };

        match w2_processing_class {
          Class::L | Class::R => last_strong_is_al = false,
          Class::AL => last_strong_is_al = true,
          _ => {}
        };

        let class_before_w456 = processing_classes[i];

        match processing_classes[i] {
          Class::EN => {
            for j in &et_run_indices {
              processing_classes[*j] = Class::EN;
            }

            et_run_indices.clear();
          }

          Class::ES | Class::CS => {
            if let Some(character) = text.get(i..).unwrap().chars().next() {
              let char_len = character.len_utf8();

              let mut next_class = self
                .iter_forwards_from(i + char_len, run_index)
                .map(|j| processing_classes[j])
                .find(|x| !x.removed_by_x9())
                .unwrap_or(self.end_class);

              if next_class == Class::EN && last_strong_is_al {
                next_class = Class::AN;
              }

              processing_classes[i] =
                match (prev_class_before_w4, processing_classes[i], next_class) {
                  (Class::EN, Class::ES, Class::EN) | (Class::EN, Class::CS, Class::EN) => {
                    Class::EN
                  }
                  (Class::AN, Class::CS, Class::AN) => Class::AN,
                  (_, _, _) => Class::ON,
                };

              if processing_classes[i] == Class::ON {
                for idx in self.iter_backwards_from(i, run_index) {
                  let class = &mut processing_classes[idx];
                  if *class != Class::BN {
                    break;
                  }

                  *class = Class::ON;
                }

                for idx in self.iter_forwards_from(i + char_len, run_index) {
                  let class = &mut processing_classes[idx];
                  if *class != Class::BN {
                    break;
                  }

                  *class = Class::ON;
                }
              }
            } else {
              processing_classes[i] = processing_classes[i - 1];
            }
          }

          Class::ET => match prev_class_before_w5 {
            Class::EN => processing_classes[i] = Class::EN,
            _ => {
              et_run_indices.extend(&bn_run_indices);
              et_run_indices.push(i);
            }
          },
          _ => {}
        };

        bn_run_indices.clear();
        prev_class_before_w5 = processing_classes[i];

        if prev_class_before_w5 != Class::ET {
          for j in &et_run_indices {
            processing_classes[*j] = Class::ON;
          }

          et_run_indices.clear();
        }

        prev_class_before_w4 = class_before_w456;
      }
    }

    for j in &et_run_indices {
      processing_classes[*j] = Class::ON;
    }

    et_run_indices.clear();

    let mut last_strong_is_l = self.start_class == Class::L;

    for run in &self.runs {
      for i in run.clone() {
        match processing_classes[i] {
          Class::EN if last_strong_is_l => processing_classes[i] = Class::L,
          Class::L => last_strong_is_l = true,
          Class::R | Class::AL => last_strong_is_l = false,
          _ => {}
        };
      }
    }
  }

  pub(crate) fn identify_bracket_pairs(
    &self,
    text: &str,
    original_classes: &[Class],
  ) -> Vec<BracketPair> {
    let mut ret = Vec::new();
    let mut stack = Vec::new();

    for (run_index, level_run) in self.runs.iter().enumerate() {
      for (i, ch) in sliced(text, level_run.clone()).char_indices() {
        let actual_index = level_run.start + i;

        if original_classes[actual_index] != Class::ON {
          continue;
        }

        if let Some(matched) = OpeningBracket::new(ch as _) {
          if matched.is_open {
            if stack.len() >= 63 {
              break;
            }

            stack.push((matched.opening, actual_index, run_index))
          } else {
            for (stack_index, element) in stack.iter().enumerate().rev() {
              if element.0 == matched.opening {
                ret.push(BracketPair {
                  start: element.1,
                  end: actual_index,
                  start_run: element.2,
                  end_run: run_index,
                });

                stack.truncate(stack_index);
                break;
              }
            }
          }
        }
      }
    }

    ret.sort_by_key(|r| r.start);
    ret
  }

  pub(crate) fn resolve_implicit_neutral(
    &self,
    text: &str,
    original_classes: &[Class],
    processing_classes: &mut [Class],
    levels: &[Level],
  ) {
    let e = levels[self.runs[0].start].class();

    let not_e = if e == Class::L { Class::R } else { Class::L };

    for pair in self.identify_bracket_pairs(text, processing_classes) {
      let mut found_e = false;
      let mut found_not_e = false;
      let mut class_to_set = None;

      let start_char_len = sliced(text, pair.start..pair.end)
        .chars()
        .next()
        .unwrap()
        .len_utf8();

      for enclosed_i in self.iter_forwards_from(pair.start + start_char_len, pair.start_run) {
        if enclosed_i >= pair.end {
          break;
        }

        let class = processing_classes[enclosed_i];

        if class == e {
          found_e = true;
        } else if class == not_e {
          found_not_e = true;
        } else if class == Class::EN || class == Class::AN {
          if e == Class::L {
            found_not_e = true;
          } else {
            found_e = true;
          }
        }

        if found_e {
          break;
        }
      }

      if found_e {
        class_to_set = Some(e);
      } else if found_not_e {
        let mut previous_strong = self
          .iter_backwards_from(pair.start, pair.start_run)
          .map(|i| processing_classes[i])
          .find(|class| {
            *class == Class::L || *class == Class::R || *class == Class::EN || *class == Class::AN
          })
          .unwrap_or(self.start_class);

        if previous_strong == Class::EN || previous_strong == Class::AN {
          previous_strong = Class::R;
        }

        class_to_set = Some(previous_strong);
      }

      if let Some(class_to_set) = class_to_set {
        let end_char_len = sliced(text, pair.end..text.len())
          .chars()
          .next()
          .unwrap()
          .len_utf8();

        for class in &mut processing_classes[pair.start..pair.start + start_char_len] {
          *class = class_to_set;
        }

        for class in &mut processing_classes[pair.end..pair.end + end_char_len] {
          *class = class_to_set;
        }

        for idx in self.iter_backwards_from(pair.start, pair.start_run) {
          let class = &mut processing_classes[idx];

          if *class != Class::BN {
            break;
          }

          *class = class_to_set;
        }

        let nsm_start = pair.start + start_char_len;

        for idx in self.iter_forwards_from(nsm_start, pair.start_run) {
          let class = original_classes[idx];

          if processing_classes[idx] == Class::BN {
            processing_classes[idx] = class_to_set;
          } else {
            break;
          }
        }

        let nsm_end = pair.end + end_char_len;

        for idx in self.iter_forwards_from(nsm_end, pair.end_run) {
          let class = original_classes[idx];

          if processing_classes[idx] == Class::BN {
            processing_classes[idx] = class_to_set;
          } else {
            break;
          }
        }
      }
    }

    let mut indices = self.runs.iter().flat_map(Clone::clone);
    let mut prev_class = self.start_class;

    while let Some(mut i) = indices.next() {
      let mut ni_run = Vec::new();

      if processing_classes[i].is_neutral_or_isolate() || processing_classes[i] == Class::BN {
        ni_run.push(i);
        let mut next_class;

        loop {
          match indices.next() {
            Some(j) => {
              i = j;
              next_class = processing_classes[j];

              if next_class.is_neutral_or_isolate() || next_class == Class::BN {
                ni_run.push(i);
              } else {
                break;
              }
            }

            None => {
              next_class = self.end_class;
              break;
            }
          };
        }

        let new_class = match (prev_class, next_class) {
          (Class::L, Class::L) => Class::L,
          (Class::R, Class::R)
          | (Class::R, Class::AN)
          | (Class::R, Class::EN)
          | (Class::AN, Class::R)
          | (Class::AN, Class::AN)
          | (Class::AN, Class::EN)
          | (Class::EN, Class::R)
          | (Class::EN, Class::AN)
          | (Class::EN, Class::EN) => Class::R,
          (_, _) => e,
        };

        for j in &ni_run {
          processing_classes[*j] = new_class;
        }

        ni_run.clear();
      }

      prev_class = processing_classes[i];
    }
  }

  fn iter_forwards_from(
    &self,
    pos: usize,
    level_run_index: usize,
  ) -> impl Iterator<Item = usize> + '_ {
    let runs = &self.runs[level_run_index..];

    (pos..runs[0].end).chain(runs[1..].iter().flat_map(Clone::clone))
  }

  fn iter_backwards_from(
    &self,
    pos: usize,
    level_run_index: usize,
  ) -> impl Iterator<Item = usize> + '_ {
    let prev_runs = &self.runs[..level_run_index];
    let current = &self.runs[level_run_index];

    (current.start..pos)
      .rev()
      .chain(prev_runs.iter().rev().flat_map(Clone::clone))
  }
}

pub(crate) struct Paragraph {
  pub(crate) range: Range<usize>,
  pub(crate) level: Level,
  pub(crate) pure_ltr: bool,
}

impl Paragraph {
  #[inline(always)]
  fn sliced<'a, T: Index<Range<usize>> + ?Sized>(
    &'a self,
    slicable: &'a T,
  ) -> &'a <T as Index<Range<usize>>>::Output {
    sliced(slicable, self.range.clone())
  }

  #[inline(always)]
  fn sliced_mut<'a, T: IndexMut<Range<usize>> + ?Sized>(
    &'a self,
    slicable: &'a mut T,
  ) -> &'a mut <T as Index<Range<usize>>>::Output {
    sliced_mut(slicable, self.range.clone())
  }

  pub(crate) fn visual_runs(
    &self,
    text: &str,
    original_classes: &[Class],
    levels: &[Level],
  ) -> Option<(Vec<Level>, Vec<Range<usize>>)> {
    let original_classes_slice = self.sliced(original_classes);
    let text = self.sliced(text);
    let mut levels = Vec::from(levels);

    let mut reset_from: Option<usize> = Some(0);
    let mut reset_to: Option<usize> = None;
    let mut prev_level = self.level;

    for (i, c) in text.char_indices() {
      match original_classes_slice[i] {
        Class::B | Class::S => {
          reset_to = Some(i + c.len_utf8());

          if reset_from.is_none() {
            reset_from = Some(i);
          }
        }

        Class::WS | Class::FSI | Class::LRI | Class::RLI | Class::PDI => {
          if reset_from.is_none() {
            reset_from = Some(i);
          }
        }

        Class::RLE | Class::LRE | Class::RLO | Class::LRO | Class::PDF | Class::BN => {
          if reset_from.is_none() {
            reset_from = Some(i);
          }

          levels[i] = prev_level;
        }

        _ => {
          reset_from = None;
        }
      }

      if let (Some(from), Some(to)) = (reset_from, reset_to) {
        for level in &mut levels[from..to] {
          *level = self.level;
        }

        reset_from = None;
        reset_to = None;
      }

      prev_level = levels[i];
    }

    if let Some(from) = reset_from {
      for level in &mut levels[from..] {
        *level = self.level;
      }
    }

    let mut runs = Vec::new();
    let mut start = self.range.start;
    let mut run_level = levels[start];
    let mut min_level = run_level;
    let mut max_level = run_level;

    for (i, &new_level) in levels
      .iter()
      .enumerate()
      .take(self.range.end)
      .skip(start + 1)
    {
      if new_level != run_level {
        runs.push(start..i);

        start = i;
        run_level = new_level;

        min_level = min(run_level, min_level);
        max_level = max(run_level, max_level);
      }
    }

    runs.push(start..self.range.end);

    let run_count = runs.len();
    min_level = min_level.new_lowest_ge_rtl()?;

    while max_level >= min_level {
      let mut seq_start = 0;

      while seq_start < run_count {
        if levels[runs[seq_start].start] < max_level {
          seq_start += 1;
          continue;
        }

        let mut seq_end = seq_start + 1;

        while seq_end < run_count {
          if levels[runs[seq_end].start] < max_level {
            break;
          }

          seq_end += 1;
        }

        runs[seq_start..seq_end].reverse();
        seq_start = seq_end;
      }

      max_level.lower(1)?;
    }

    Some((levels, runs))
  }

  pub(crate) fn compute_explicit(
    &self,
    input: &str,
    original_classes: &[Class],
    processing_classes: &mut [Class],
    levels: &mut [Level],
  ) {
    let input_slice = self.sliced(input);
    let original_classes_slice = self.sliced(original_classes);
    let processing_classes_slice = self.sliced_mut(processing_classes);
    let levels_slice = self.sliced_mut(levels);

    let mut stack = Vec::with_capacity(1);

    stack.push(Status {
      level: self.level,
      status: OverrideStatus::Neutral,
    });

    let mut overflow_isolate_count = 0;
    let mut overflow_embedding_count = 0;
    let mut valid_isolate_count = 0;

    for (idx, character) in input_slice.char_indices() {
      let current_class = original_classes_slice[idx];

      match current_class {
        Class::RLE
        | Class::LRE
        | Class::RLO
        | Class::LRO
        | Class::RLI
        | Class::LRI
        | Class::FSI => {
          let last = stack.last().unwrap();
          levels_slice[idx] = last.level;

          let is_isolate = current_class.is_isolate();

          if is_isolate {
            match last.status {
              OverrideStatus::RTL => processing_classes_slice[idx] = Class::R,
              OverrideStatus::LTR => processing_classes_slice[idx] = Class::L,
              _ => {}
            }
          }

          let new_level = if current_class.is_rtl() {
            last.level.new_explicit_next_rtl()
          } else {
            last.level.new_explicit_next_ltr()
          };

          if new_level.is_some() && overflow_isolate_count == 0 && overflow_embedding_count == 0 {
            // SAFETY: new_level was already proven to be Some
            let new_level = unsafe { new_level.unwrap_unchecked() };

            stack.push(Status {
              level: new_level,
              status: original_classes_slice[idx].override_status(),
            });

            if is_isolate {
              valid_isolate_count += 1;
            } else {
              levels_slice[idx] = new_level;
            }
          } else if is_isolate {
            overflow_isolate_count += 1;
          } else if overflow_isolate_count == 0 {
            overflow_embedding_count += 1;
          }

          if !is_isolate {
            processing_classes_slice[idx] = Class::BN;
          }
        }

        Class::PDI => {
          if overflow_isolate_count > 0 {
            overflow_isolate_count -= 1;
          } else if valid_isolate_count > 0 {
            overflow_embedding_count = 0;

            loop {
              match stack.pop() {
                None
                | Some(Status {
                  status: OverrideStatus::Isolate,
                  ..
                }) => break,
                _ => continue,
              }
            }

            valid_isolate_count -= 1;
          }

          let last = stack.last().unwrap();
          levels_slice[idx] = last.level;

          match last.status {
            OverrideStatus::RTL => processing_classes_slice[idx] = Class::R,
            OverrideStatus::LTR => processing_classes_slice[idx] = Class::L,
            _ => {}
          }
        }

        Class::PDF => {
          if overflow_isolate_count <= 0 {
            if overflow_embedding_count > 0 {
              overflow_embedding_count -= 1;
            } else if stack.last().unwrap().status != OverrideStatus::Isolate && stack.len() >= 2 {
              stack.pop();
            }
          }

          levels_slice[idx] = stack.last().unwrap().level;
          processing_classes_slice[idx] = Class::BN;
        }

        Class::B => {}

        _ => {
          let last = stack.last().unwrap();
          levels_slice[idx] = last.level;

          if current_class != Class::BN {
            match last.status {
              OverrideStatus::RTL => processing_classes_slice[idx] = Class::R,
              OverrideStatus::LTR => processing_classes_slice[idx] = Class::L,
              _ => {}
            }
          }
        }
      }

      for j in 1..character.len_utf8() {
        levels_slice[idx + j] = levels_slice[idx];
        processing_classes_slice[idx + j] = processing_classes_slice[idx];
      }
    }
  }

  pub(crate) fn isolating_run_sequences<'a>(
    &'a self,
    levels: &'a [Level],
    original_classes: &'a [Class],
  ) -> impl Iterator<Item = IsolatingRunSequence> + 'a {
    let mut runs = Vec::new();

    if !levels.is_empty() {
      let mut current_run_level = levels[0];
      let mut current_run_start = 0;

      for i in 1..levels.len() {
        if !original_classes[i].removed_by_x9() && levels[i] != current_run_level {
          runs.push(current_run_start..i);

          current_run_level = levels[i];
          current_run_start = i;
        }
      }

      runs.push(current_run_start..levels.len());
    }

    let mut sequences = Vec::with_capacity(runs.len());
    let mut stack = Vec::with_capacity(1);
    stack.push(Vec::new());

    for run in runs {
      let start_class = original_classes[run.start];
      let end_class = original_classes[run.clone()]
        .iter()
        .copied()
        .rev()
        .filter(|x| !x.removed_by_x9())
        .next()
        .unwrap_or(start_class);

      let mut sequence = if start_class == Class::PDI && stack.len() > 1 {
        // SAFETY: stack is already checked to be not empty
        unsafe { stack.pop().unwrap_unchecked() }
      } else {
        Vec::new()
      };

      sequence.push(run);

      if end_class.is_isolate() {
        stack.push(sequence);
      } else {
        sequences.push(sequence);
      }
    }

    sequences.extend(stack.into_iter().rev().filter(|seq| !seq.is_empty()));

    sequences.into_iter().map(|sequence| {
      let mut result = IsolatingRunSequence {
        runs: sequence,
        start_class: Class::L,
        end_class: Class::L,
      };

      let sequence_start = result.runs[0].start;
      let runs_len = result.runs.len();
      let sequence_end = result.runs[runs_len - 1].end;

      let sequence_level = result
        .iter_forwards_from(sequence_start, 0)
        .filter(|i| !original_classes[*i].removed_by_x9())
        .map(|i| levels[i])
        .next()
        .unwrap_or(levels[sequence_start]);

      let end_level = result
        .iter_backwards_from(sequence_end, runs_len - 1)
        .filter(|i| !original_classes[*i].removed_by_x9())
        .map(|i| levels[i])
        .next()
        .unwrap_or(levels[sequence_end - 1]);

      let preceeding_level = match original_classes[..sequence_start]
        .iter()
        .rposition(|x| !x.removed_by_x9())
      {
        Some(idx) => levels[idx],
        None => self.level,
      };

      let last_non_removed = original_classes[..sequence_end]
        .iter()
        .copied()
        .rev()
        .find(|x| !x.removed_by_x9())
        .unwrap_or(Class::BN);

      let succeeding_level = if last_non_removed.is_isolate() {
        self.level
      } else {
        match original_classes[sequence_end..]
          .iter()
          .position(|x| !x.removed_by_x9())
        {
          Some(idx) => levels[sequence_end + idx],
          None => self.level,
        }
      };

      result.start_class = max(sequence_level, preceeding_level).class();
      result.end_class = max(end_level, succeeding_level).class();
      result
    })
  }
}
