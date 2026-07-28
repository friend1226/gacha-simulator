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
        debug_assert!(index < self.state_space);
        let mut remaining = index;
        let mut control = Vec::with_capacity(self.control_max.len());
        let mut counts = Vec::with_capacity(self.count_max.len());
        for maximum in &self.control_max {
            let modulus = u64::from(*maximum) + 1;
            control.push((remaining % modulus) as u32);
            remaining /= modulus;
        }
        for maximum in &self.count_max {
            let modulus = u64::from(*maximum) + 1;
            counts.push((remaining % modulus) as u32);
            remaining /= modulus;
        }
        (control, counts)
    }

    pub fn control_index(&self, index: u64) -> usize {
        (index % self.control_states) as usize
    }

    pub fn state_space(&self) -> u64 {
        self.state_space
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
