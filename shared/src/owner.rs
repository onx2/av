/// Globally-unique identifier for any "data owner" that can have ephemeral component rows
/// (e.g. Character/Npc/Monster).
///
/// # Why this exists
/// SpacetimeDB currently restricts primary keys / indexes to primitive scalar column types.
/// We cannot use a composite primary key like `(OwnerKind, owner_id)` or a custom struct type
/// as an indexed key. To keep a single-column primary key while preventing cross-table ID
/// collisions, owner_id and OwnerKind are packed into a single `u128`.
///
/// # Bit layout
/// This `u128` is a packed value with the following layout (least-significant bit = bit 0):
///
/// - bits 0..=63   : `owner_id` (u64)
/// - bits 64..=71  : `OwnerKind` tag (u8)
/// - bits 72..=127 : reserved (must be zero for now)
///
/// # Invariants
/// - Two different `(owner_id, kind)` pairs must never produce the same `Owner`.
/// - Reserved bits must remain zero (until a versioned migration is introduced).
///
/// # Compatibility
/// Treat the bit layout as a wire/storage format. Changing it requires a data migration
pub type Owner = u128;

/// The primary_key (unique ID) used for a specific kind of owner (e.g. Character, Monster, NPC)
pub type OwnerId = u64;

/// A generic way to retrieve the unpacked owner data
pub trait AsOwner {
    fn owner(&self) -> Owner;
    fn owner_id(&self) -> OwnerId;
    fn owner_kind(&self) -> OwnerKind;
}

/// Discriminator for the kind of entity referenced by an [`Owner`].
///
/// The numeric values of this enum are part of the packed-ID storage format. Do not reorder
/// or reuse values without a migration.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OwnerKind {
    Character = 1,
    Npc = 2,
    Monster = 3,
}

/// Packs an [`OwnerKind`] and a per-kind `owner_id` into a globally-unique [`Owner`].
///
/// # Panics / Errors
/// This function assumes `id` fits in [OwnerId] BITS by construction and `kind` will fit in the remaining bits.
///
/// # Examples
/// ```text
/// let owner = pack_owner(42, OwnerKind::Character);
/// assert_eq!(unpack_owner_kind(owner), Some(OwnerKind::Character));
/// assert_eq!(unpack_owner_id(owner), 42);
/// ```
pub fn pack_owner(id: OwnerId, kind: OwnerKind) -> Owner {
    (id as u128) | ((kind as u128) << OwnerId::BITS)
}

/// Extracts the [OwnerKind] encoded in an [Owner].
///
/// # Panics / Errors
/// This function assumes the `kind` is a valid u8 following the # of [OwnerId] BITS

pub fn unpack_owner_kind(owner: Owner) -> OwnerKind {
    try_unpack_owner_kind(owner).expect("Unsupported OwnerKind.")
}
/// Safely extracts the [OwnerKind] from an [`Owner`].
///
/// Returns `None` if the tag is unknown (e.g. data corruption, future kinds, or mismatched
/// packing rules across code versions).
///
/// Prefer handling `None` by failing fast in reducers, since an unknown kind means you
/// cannot safely interpret the remaining fields.
pub fn try_unpack_owner_kind(owner: Owner) -> Option<OwnerKind> {
    const KIND_MASK: u128 = u8::MAX as u128;
    let tag = ((owner >> OwnerId::BITS) & KIND_MASK) as u8;

    match tag {
        1u8 => Some(OwnerKind::Character),
        2u8 => Some(OwnerKind::Npc),
        3u8 => Some(OwnerKind::Monster),
        _ => None,
    }
}

/// Extracts the [OwnerId] from an [`Owner`].
///
/// Note: this does not validate the kind tag.
pub fn unpack_owner_id(owner: Owner) -> OwnerId {
    const ID_MASK: u128 = u64::MAX as u128;
    (owner & ID_MASK) as OwnerId
}

/// Validates that an [`Owner`] conforms to the current packing contract.
///
/// Use this at boundaries (e.g. reducer inputs) to fail fast on corrupted or mismatched IDs.
///
/// Checks:
/// - kind tag is recognized
/// - reserved bits (40..=63) are zero
pub fn validate_owner_id(owner: Owner) -> Result<(), &'static str> {
    const RESERVED_MASK: u128 = !0u128 << 72; // bits 72..127 set
    if (owner & RESERVED_MASK) != 0 {
        return Err("Owner reserved bits are non-zero");
    }
    if try_unpack_owner_kind(owner).is_none() {
        return Err("Owner has unknown kind tag");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_unpacks_owner_id_and_kind() {
        let ids: [OwnerId; 6] = [0, 1, 42, 1337, u32::MAX as u64, u64::MAX];
        let kinds = [OwnerKind::Character, OwnerKind::Npc, OwnerKind::Monster];

        for &id in &ids {
            for &kind in &kinds {
                let owner = pack_owner(id, kind);

                assert_eq!(unpack_owner_id(owner), id);
                assert_eq!(unpack_owner_kind(owner), kind);
                assert_eq!(try_unpack_owner_kind(owner), Some(kind));

                // Validate should pass for properly-packed values.
                assert_eq!(validate_owner_id(owner), Ok(()));
            }
        }
    }

    #[test]
    fn pack_places_id_in_low_64_bits_and_kind_in_next_8_bits() {
        let id: OwnerId = 0x0123_4567_89AB_CDEF;
        let kind = OwnerKind::Monster;

        let owner = pack_owner(id, kind);

        let expected = (id as u128) | ((kind as u128) << 64);
        assert_eq!(owner, expected);

        // Sanity: upper reserved bits should be zero in normal packing.
        assert_eq!((owner >> 72) as u64, 0);
    }

    #[test]
    fn try_unpack_returns_none_for_unknown_kind() {
        let id: OwnerId = 123;
        let unknown_tag: u8 = 255;

        let owner: Owner = (id as u128) | ((unknown_tag as u128) << 64);

        assert_eq!(unpack_owner_id(owner), id);
        assert_eq!(try_unpack_owner_kind(owner), None);
    }

    #[test]
    #[should_panic(expected = "Unsupported OwnerKind.")]
    fn unpack_owner_kind_panics_for_unknown_kind() {
        let id: OwnerId = 123;
        let unknown_tag: u8 = 200;

        let owner: Owner = (id as u128) | ((unknown_tag as u128) << 64);

        let _ = unpack_owner_kind(owner);
    }

    #[test]
    fn validate_fails_if_reserved_bits_non_zero() {
        let owner = pack_owner(42, OwnerKind::Character) | (1u128 << 72);

        assert_eq!(
            validate_owner_id(owner),
            Err("Owner reserved bits are non-zero")
        );
    }

    #[test]
    fn validate_fails_if_kind_unknown_even_if_reserved_bits_zero() {
        let id: OwnerId = 42;
        let unknown_tag: u8 = 250;
        let owner: Owner = (id as u128) | ((unknown_tag as u128) << 64);

        assert_eq!(validate_owner_id(owner), Err("Owner has unknown kind tag"));
    }
}
