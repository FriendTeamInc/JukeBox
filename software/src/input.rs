// Defining inputs that the device recieves

use std::{collections::HashSet, fmt};

use jukebox_util::peripheral::{KeyInputs, KnobInputs, PedalInputs};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Hash, Clone, Copy)]
pub enum InputKey {
    UnknownKey,

    KeySwitch1,
    KeySwitch2,
    KeySwitch3,
    KeySwitch4,
    KeySwitch5,
    KeySwitch6,
    KeySwitch7,
    KeySwitch8,
    KeySwitch9,
    KeySwitch10,
    KeySwitch11,
    KeySwitch12,
    KeySwitch13,
    KeySwitch14,
    KeySwitch15,
    KeySwitch16,

    KnobLeftSwitch,
    KnobLeftClockwise,
    KnobLeftCounterClockwise,
    KnobRightSwitch,
    KnobRightClockwise,
    KnobRightCounterClockwise,

    PedalLeft,
    PedalMiddle,
    PedalRight,
}
impl InputKey {
    pub fn trans_keys(i: KeyInputs) -> HashSet<Self> {
        let mut res = HashSet::new();

        let mut doif = |c, f| {
            if c {
                res.insert(f);
            }
        };

        doif(i.key1.is_down(), Self::KeySwitch1);
        doif(i.key2.is_down(), Self::KeySwitch2);
        doif(i.key3.is_down(), Self::KeySwitch3);
        doif(i.key4.is_down(), Self::KeySwitch4);
        doif(i.key5.is_down(), Self::KeySwitch5);
        doif(i.key6.is_down(), Self::KeySwitch6);
        doif(i.key7.is_down(), Self::KeySwitch7);
        doif(i.key8.is_down(), Self::KeySwitch8);
        doif(i.key9.is_down(), Self::KeySwitch9);
        doif(i.key10.is_down(), Self::KeySwitch10);
        doif(i.key11.is_down(), Self::KeySwitch11);
        doif(i.key12.is_down(), Self::KeySwitch12);
        doif(i.key13.is_down(), Self::KeySwitch13);
        doif(i.key14.is_down(), Self::KeySwitch14);
        doif(i.key15.is_down(), Self::KeySwitch15);
        doif(i.key16.is_down(), Self::KeySwitch16);

        res
    }

    pub fn trans_knob(i: KnobInputs) -> HashSet<Self> {
        let mut res = HashSet::new();

        let mut doif = |c, f| {
            if c {
                res.insert(f);
            }
        };

        doif(i.left_switch.is_down(), Self::KnobLeftSwitch);
        doif(i.left_direction.is_clockwise(), Self::KnobLeftClockwise);
        doif(
            i.left_direction.is_counter_clockwise(),
            Self::KnobLeftCounterClockwise,
        );

        doif(i.right_switch.is_down(), Self::KnobRightSwitch);
        doif(i.right_direction.is_clockwise(), Self::KnobRightClockwise);
        doif(
            i.right_direction.is_counter_clockwise(),
            Self::KnobRightCounterClockwise,
        );

        res
    }

    pub fn trans_pedals(i: PedalInputs) -> HashSet<Self> {
        let mut res = HashSet::new();

        let mut doif = |c, f| {
            if c {
                res.insert(f);
            }
        };

        doif(i.left.is_down(), Self::PedalLeft);
        doif(i.middle.is_down(), Self::PedalMiddle);
        doif(i.right.is_down(), Self::PedalRight);

        res
    }
}
impl fmt::Display for InputKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
