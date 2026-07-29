use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum StateCodecError {
    #[error("state dimension count does not match codec")]
    DimensionMismatch,
    #[error("state value {value} exceeds dimension maximum {maximum}")]
    ValueOutOfRange { value: u32, maximum: u32 },
    #[error("mixed-radix state space exceeds u64")]
    StateSpaceOverflow,
}

#[derive(Debug, Clone)]
pub struct StateCodec {
    control_max: Vec<u32>,
    accumulator_max: Vec<u32>,
    count_max: Vec<u32>,
    strides: Vec<u64>,
    control_states: u64,
    accumulator_offset: usize,
    count_offset: usize,
    state_space: u64,
}

impl StateCodec {
    pub fn new(control_max: &[u32], count_max: &[u32]) -> Result<Self, StateCodecError> {
        Self::with_accumulators(control_max, &[], count_max)
    }

    pub fn with_accumulators(
        control_max: &[u32],
        accumulator_max: &[u32],
        count_max: &[u32],
    ) -> Result<Self, StateCodecError> {
        let mut strides =
            Vec::with_capacity(control_max.len() + accumulator_max.len() + count_max.len());
        let mut state_space = 1u64;
        for maximum in control_max.iter().chain(accumulator_max).chain(count_max) {
            strides.push(state_space);
            state_space = state_space
                .checked_mul(u64::from(*maximum) + 1)
                .ok_or(StateCodecError::StateSpaceOverflow)?;
        }
        let control_states = control_max.iter().try_fold(1u64, |states, maximum| {
            states
                .checked_mul(u64::from(*maximum) + 1)
                .ok_or(StateCodecError::StateSpaceOverflow)
        })?;
        Ok(Self {
            control_max: control_max.to_vec(),
            accumulator_max: accumulator_max.to_vec(),
            count_max: count_max.to_vec(),
            strides,
            control_states,
            accumulator_offset: control_max.len(),
            count_offset: control_max.len() + accumulator_max.len(),
            state_space,
        })
    }

    pub fn encode(&self, control: &[u32], counts: &[u32]) -> Result<u64, StateCodecError> {
        self.encode_full(control, &[], counts)
    }

    pub fn encode_full(
        &self,
        control: &[u32],
        accumulators: &[u32],
        counts: &[u32],
    ) -> Result<u64, StateCodecError> {
        if control.len() != self.control_max.len()
            || accumulators.len() != self.accumulator_max.len()
            || counts.len() != self.count_max.len()
        {
            return Err(StateCodecError::DimensionMismatch);
        }
        let mut index = 0u64;
        for ((value, maximum), stride) in control
            .iter()
            .chain(accumulators)
            .chain(counts)
            .zip(
                self.control_max
                    .iter()
                    .chain(&self.accumulator_max)
                    .chain(&self.count_max),
            )
            .zip(&self.strides)
        {
            if value > maximum {
                return Err(StateCodecError::ValueOutOfRange {
                    value: *value,
                    maximum: *maximum,
                });
            }
            index += u64::from(*value) * stride;
        }
        Ok(index)
    }

    pub fn decode(&self, index: u64) -> (Vec<u32>, Vec<u32>) {
        let mut control = vec![0; self.control_max.len()];
        let mut accumulators = vec![0; self.accumulator_max.len()];
        let mut counts = vec![0; self.count_max.len()];
        self.decode_full_into(index, &mut control, &mut accumulators, &mut counts);
        (control, counts)
    }

    pub fn decode_into(&self, index: u64, control: &mut [u32], counts: &mut [u32]) {
        let mut accumulators = vec![0; self.accumulator_max.len()];
        self.decode_full_into(index, control, &mut accumulators, counts);
    }

    pub fn decode_full(&self, index: u64) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
        let mut control = vec![0; self.control_max.len()];
        let mut accumulators = vec![0; self.accumulator_max.len()];
        let mut counts = vec![0; self.count_max.len()];
        self.decode_full_into(index, &mut control, &mut accumulators, &mut counts);
        (control, accumulators, counts)
    }

    pub fn decode_full_into(
        &self,
        index: u64,
        control: &mut [u32],
        accumulators: &mut [u32],
        counts: &mut [u32],
    ) {
        debug_assert!(index < self.state_space);
        assert_eq!(
            control.len(),
            self.control_max.len(),
            "control buffer length"
        );
        assert_eq!(
            accumulators.len(),
            self.accumulator_max.len(),
            "accumulator buffer length"
        );
        assert_eq!(counts.len(), self.count_max.len(), "count buffer length");
        let mut remaining = index;
        for (value, maximum) in control.iter_mut().zip(&self.control_max) {
            let modulus = u64::from(*maximum) + 1;
            *value = (remaining % modulus) as u32;
            remaining /= modulus;
        }
        for (value, maximum) in accumulators.iter_mut().zip(&self.accumulator_max) {
            let modulus = u64::from(*maximum) + 1;
            *value = (remaining % modulus) as u32;
            remaining /= modulus;
        }
        for (value, maximum) in counts.iter_mut().zip(&self.count_max) {
            let modulus = u64::from(*maximum) + 1;
            *value = (remaining % modulus) as u32;
            remaining /= modulus;
        }
    }

    pub fn control_index(&self, index: u64) -> usize {
        (index % self.control_states) as usize
    }

    pub fn state_space(&self) -> u64 {
        self.state_space
    }

    pub fn replace_control_index(&self, state: u64, control_index: usize) -> u64 {
        debug_assert!(
            state < self.state_space,
            "packed state is outside the codec state space"
        );
        debug_assert!(
            control_index < self.control_states as usize,
            "control index is outside the codec control state space"
        );
        state - state % self.control_states + control_index as u64
    }

    pub fn increment_count(&self, state: u64, position: usize) -> u64 {
        debug_assert!(
            state < self.state_space,
            "packed state is outside the codec state space"
        );
        debug_assert!(
            position < self.count_max.len(),
            "count position is outside the codec"
        );
        let maximum = self.count_max[position];
        let stride = self.strides[self.count_offset + position];
        let current = (state / stride) % (u64::from(maximum) + 1);
        debug_assert!(
            current < u64::from(maximum),
            "count digit overflow would carry into the next packed field"
        );
        state + stride
    }

    pub fn accumulator_value(&self, state: u64, position: usize) -> u32 {
        debug_assert!(
            state < self.state_space,
            "packed state is outside the codec state space"
        );
        debug_assert!(
            position < self.accumulator_max.len(),
            "accumulator position is outside the codec"
        );
        let stride = self.strides[self.accumulator_offset + position];
        ((state / stride) % (u64::from(self.accumulator_max[position]) + 1)) as u32
    }

    pub fn replace_accumulator_index(&self, state: u64, position: usize, value: u32) -> u64 {
        debug_assert!(
            state < self.state_space,
            "packed state is outside the codec state space"
        );
        debug_assert!(
            position < self.accumulator_max.len(),
            "accumulator position is outside the codec"
        );
        let maximum = self.accumulator_max[position];
        debug_assert!(
            value <= maximum,
            "accumulator value exceeds its packed field"
        );
        let stride = self.strides[self.accumulator_offset + position];
        let radix = u64::from(maximum) + 1;
        let current = (state / stride) % radix;
        state - current * stride + u64::from(value) * stride
    }

    pub fn control_len(&self) -> usize {
        self.control_max.len()
    }
    pub fn accumulator_len(&self) -> usize {
        self.accumulator_max.len()
    }
    pub fn count_len(&self) -> usize {
        self.count_max.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_radix_round_trips_and_keeps_control_in_low_digits() {
        let codec = StateCodec::new(&[2, 3], &[5, 7]).unwrap();
        let index = codec.encode(&[2, 1], &[4, 6]).unwrap();

        assert_eq!(codec.decode(index), (vec![2, 1], vec![4, 6]));
        assert_eq!(codec.control_index(index), 5);
        assert_eq!(codec.state_space(), 3 * 4 * 6 * 8);

        let replaced = codec.replace_control_index(index, 7);
        assert_eq!(codec.decode(replaced), (vec![1, 2], vec![4, 6]));
        let incremented = codec.increment_count(replaced, 0);
        assert_eq!(codec.decode(incremented), (vec![1, 2], vec![5, 6]));
    }

    #[test]
    fn mixed_radix_rejects_overflow_and_out_of_range_values() {
        assert_eq!(
            StateCodec::new(&[u32::MAX, u32::MAX], &[0]).unwrap_err(),
            StateCodecError::StateSpaceOverflow,
        );
        let codec = StateCodec::new(&[1], &[2]).unwrap();
        assert_eq!(
            codec.encode(&[2], &[0]),
            Err(StateCodecError::ValueOutOfRange {
                value: 2,
                maximum: 1,
            }),
        );
    }

    #[test]
    fn accumulator_digits_are_independent_from_control_and_counts() {
        let codec = StateCodec::with_accumulators(&[2], &[4, 6], &[3]).unwrap();
        let state = codec.encode_full(&[1], &[2, 5], &[3]).unwrap();
        assert_eq!(codec.decode_full(state), (vec![1], vec![2, 5], vec![3]));
        let replaced = codec.replace_accumulator_index(state, 0, 4);
        assert_eq!(codec.decode_full(replaced), (vec![1], vec![4, 5], vec![3]));
        assert_eq!(codec.control_index(replaced), 1);
    }

    #[test]
    #[should_panic(expected = "count digit overflow")]
    fn packed_count_increment_rejects_carry_into_the_next_field() {
        let codec = StateCodec::new(&[1], &[2, 3]).unwrap();
        let state = codec.encode(&[0], &[2, 1]).unwrap();

        let _ = codec.increment_count(state, 0);
    }

    #[test]
    #[should_panic(expected = "control index is outside")]
    fn packed_control_replacement_rejects_out_of_range_index() {
        let codec = StateCodec::new(&[1, 2], &[3]).unwrap();
        let state = codec.encode(&[0, 0], &[1]).unwrap();

        let _ = codec.replace_control_index(state, 6);
    }
}
