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
    count_max: Vec<u32>,
    strides: Vec<u64>,
    control_states: u64,
    state_space: u64,
}

impl StateCodec {
    pub fn new(control_max: &[u32], count_max: &[u32]) -> Result<Self, StateCodecError> {
        let mut strides = Vec::with_capacity(control_max.len() + count_max.len());
        let mut state_space = 1u64;
        for maximum in control_max.iter().chain(count_max) {
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
            count_max: count_max.to_vec(),
            strides,
            control_states,
            state_space,
        })
    }

    pub fn encode(&self, control: &[u32], counts: &[u32]) -> Result<u64, StateCodecError> {
        if control.len() != self.control_max.len() || counts.len() != self.count_max.len() {
            return Err(StateCodecError::DimensionMismatch);
        }
        let mut index = 0u64;
        for ((value, maximum), stride) in control
            .iter()
            .chain(counts)
            .zip(self.control_max.iter().chain(&self.count_max))
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
        let mut counts = vec![0; self.count_max.len()];
        self.decode_into(index, &mut control, &mut counts);
        (control, counts)
    }

    pub fn decode_into(&self, index: u64, control: &mut [u32], counts: &mut [u32]) {
        debug_assert!(index < self.state_space);
        assert_eq!(control.len(), self.control_max.len(), "control buffer length");
        assert_eq!(counts.len(), self.count_max.len(), "count buffer length");
        let mut remaining = index;
        for (value, maximum) in control.iter_mut().zip(&self.control_max) {
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
        debug_assert!(control_index < self.control_states as usize);
        state - state % self.control_states + control_index as u64
    }

    pub fn increment_count(&self, state: u64, position: usize) -> u64 {
        state + self.strides[self.control_max.len() + position]
    }

    pub fn control_len(&self) -> usize { self.control_max.len() }
    pub fn count_len(&self) -> usize { self.count_max.len() }
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
}
