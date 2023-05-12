extern crate std;

pub use iuuid::Uuid;

use core::sync::atomic::{AtomicU16, Ordering};

#[inline(always)]
fn now() -> (u64, u32) {
    let duration = std::time::SystemTime::UNIX_EPOCH.elapsed().expect("To get time");
    (duration.as_secs(), duration.subsec_nanos())
}

fn v1(mac: [u8; 6]) -> Uuid {
    use iuuid::timestamp::UUID_TICKS_BETWEEN_EPOCHS;

    static COUNTER: AtomicU16 = AtomicU16::new(1);

    let (seconds, nanos) = now();
    let ticks = UUID_TICKS_BETWEEN_EPOCHS + seconds * 10_000_000 + nanos as u64 / 100;

    let counter = COUNTER.fetch_add(1, Ordering::AcqRel) % (u16::MAX >> 2);

    //v1 is very useful to generate unique uuid among multiple nodes
    iuuid::Builder::from_rfc4122_timestamp(ticks, counter, &mac).into_uuid()
}

fn v4(_: [u8; 6]) -> Uuid {
    Uuid::new_v4()
}

const V1: fn([u8; 6]) -> Uuid = v1;
const V4: fn([u8; 6]) -> Uuid = v4;

#[derive(Copy, Clone)]
///Generator which by default uses `v1` and fallbacks to `v4` if mac address is unknown
pub struct UuidGenerator {
    mac: [u8; 6],
    gen: fn([u8; 6]) -> Uuid
}

impl UuidGenerator {
    ///Creates random based uuid generator.
    pub const fn new_v4() -> Self {
        Self {
            mac: [0; 6],
            gen: V4,
        }
    }

    ///Creates new instance.
    ///
    ///If mac address is available, generator will use `uuid` v1.
    ///Otherwise it defaults to `v4`
    ///
    ///In case that is not desirable please use `new_v4` to only use random generator.
    pub fn new() -> Self {
        let (mac, gen) = match mac_address::get_mac_address() {
            Ok(Some(addr)) => (addr.bytes(), V1),
            //It is generally ok to use v4 as it is unique enough
            _ => ([0; 6], V4)
        };

        Self {
            mac,
            gen
        }
    }

    #[inline(always)]
    ///Returns whether generate is able to use `v1`
    pub fn is_v1(&self) -> bool {
        self.gen == V1
    }

    #[inline(always)]
    ///Generates `UUID`
    pub fn gen(&self) -> Uuid {
        (self.gen)(self.mac)
    }
}

impl super::IdGen<Uuid> for UuidGenerator {
    #[inline(always)]
    fn gen(&self) -> Uuid {
        Self::gen(self)
    }
}

impl super::IdGen<alloc::string::String> for UuidGenerator {
    #[inline(always)]
    fn gen(&self) -> alloc::string::String {
        alloc::format!("{}", Self::gen(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_v1_is_used_when_mac_avail() {
        let expected = match mac_address::get_mac_address() {
            Ok(Some(_)) => true,
            _ => false,
        };

        let generator = UuidGenerator::new();
        assert_eq!(expected, generator.is_v1());
    }

    #[test]
    fn should_generate_unique_uuid() {
        let uuid = UuidGenerator::new();

        let mut prev = uuid.gen();
        for _ in 0..101 {
            let next = uuid.gen();
            assert_ne!(prev, next);
            prev = next;
        }
    }
}
