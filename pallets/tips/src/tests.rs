// This file is part of Deeper.

// Copyright (C) 2020-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Treasury pallet tests.

#![cfg(test)]

use std::cell::RefCell;

use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BadOrigin, BlakeTwo256, IdentityLookup},
    Perbill, Permill,
};
use sp_storage::Storage;

use frame_support::{
    assert_noop, assert_ok, pallet_prelude::GenesisBuild, parameter_types,
    storage::StoragePrefixedMap, traits::SortedMembers, weights::Weight, PalletId,
};

use super::*;
use crate::{self as pallet_tips, Event as TipEvent};
use frame_system::RawOrigin;
use pallet_credit::{CreditData, CreditLevel};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Treasury: pallet_treasury::{Pallet, Call, Storage, Config, Event<T>},
        Tips: pallet_tips::{Pallet, Call, Storage, Event<T>},
        Credit: pallet_credit::{Pallet, Call, Storage, Event<T>, Config<T>},
        DeeperNode: pallet_deeper_node::{Pallet, Call, Storage, Event<T>, Config<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Call = Call;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u128; // u64 is not enough to hold bytes used to generate bounty account
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
}
parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}
impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type Balance = u64;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}
thread_local! {
    static TEN_TO_FOURTEEN: RefCell<Vec<u128>> = RefCell::new(vec![10,11,12,13,14]);
}
pub struct TenToFourteen;
impl SortedMembers<u128> for TenToFourteen {
    fn sorted_members() -> Vec<u128> {
        TEN_TO_FOURTEEN.with(|v| v.borrow().clone())
    }
    #[cfg(feature = "runtime-benchmarks")]
    fn add(new: &u128) {
        TEN_TO_FOURTEEN.with(|v| {
            let mut members = v.borrow_mut();
            members.push(*new);
            members.sort();
        })
    }
}
impl ContainsLengthBound for TenToFourteen {
    fn max_len() -> usize {
        TEN_TO_FOURTEEN.with(|v| v.borrow().len())
    }
    fn min_len() -> usize {
        0
    }
}
parameter_types! {
    pub const ProposalBond: Permill = Permill::from_percent(5);
    pub const ProposalBondMinimum: u64 = 1;
    pub const SpendPeriod: u64 = 2;
    pub const Burn: Permill = Permill::from_percent(50);
    pub const DataDepositPerByte: u64 = 1;
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub const MaximumReasonLength: u32 = 16384;
    pub const MaxApprovals: u32 = 100;
}
impl pallet_treasury::Config for Test {
    type PalletId = TreasuryPalletId;
    type Currency = pallet_balances::Pallet<Test>;
    type ApproveOrigin = frame_system::EnsureRoot<u128>;
    type RejectOrigin = frame_system::EnsureRoot<u128>;
    type Event = Event;
    type OnSlash = ();
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ProposalBondMinimum;
    type SpendPeriod = SpendPeriod;
    type Burn = Burn;
    type BurnDestination = (); // Just gets burned.
    type WeightInfo = ();
    type SpendFunds = ();
    type MaxApprovals = MaxApprovals;
}
parameter_types! {
    pub const CreditCapTwoEras: u8 = 5;
    pub const CreditAttenuationStep: u64 = 1;
    pub const MinCreditToDelegate: u64 = 100;
    pub const MicropaymentToCreditFactor: u128 = 1_000_000_000_000_000;
    pub const BlocksPerEra: u64 =  17280;
    pub const SecsPerBlock: u32 = 5u32;
}
impl pallet_credit::Config for Test {
    type Event = Event;
    type BlocksPerEra = BlocksPerEra;
    type Currency = Balances;
    type CreditCapTwoEras = CreditCapTwoEras;
    type CreditAttenuationStep = CreditAttenuationStep;
    type MinCreditToDelegate = MinCreditToDelegate;
    type MicropaymentToCreditFactor = MicropaymentToCreditFactor;
    type NodeInterface = DeeperNode;
    type WeightInfo = ();
    type UnixTime = Timestamp;
    type SecsPerBlock = SecsPerBlock;
}
parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}
parameter_types! {
    pub const MinLockAmt: u32 = 100;
    pub const MaxDurationEras: u8 = 7;
    pub const MaxIpLength: usize = 256;
}
impl pallet_deeper_node::Config for Test {
    type Event = Event;
    type Currency = Balances;
    type MinLockAmt = MinLockAmt;
    type MaxDurationEras = MaxDurationEras;
    type BlocksPerEra = BlocksPerEra;
    type MaxIpLength = MaxIpLength;
    type WeightInfo = ();
}
parameter_types! {
    pub const TipCountdown: u64 = 1;
    pub const TipFindersFee: Percent = Percent::from_percent(20);
    pub const TipReportDepositBase: u64 = 1;
    pub const MaximumCreditReward: u64 = 15;
}
impl Config for Test {
    type MaximumReasonLength = MaximumReasonLength;
    type MaximumCreditReward = MaximumCreditReward;
    type CreditInterface = Credit;
    type Tippers = TenToFourteen;
    type TipCountdown = TipCountdown;
    type TipFindersFee = TipFindersFee;
    type TipReportDepositBase = TipReportDepositBase;
    type DataDepositPerByte = DataDepositPerByte;
    type Event = Event;
    type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    pallet_balances::GenesisConfig::<Test> {
        // Total issuance will be 200 with treasury account initialized at ED.
        balances: vec![(0, 100), (1, 98), (2, 1)],
    }
    .assimilate_storage(&mut t)
    .unwrap();
    GenesisBuild::<Test>::assimilate_storage(&pallet_treasury::GenesisConfig, &mut t).unwrap();
    t.into()
}

fn last_event() -> TipEvent<Test> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let Event::Tips(inner) = e {
                Some(inner)
            } else {
                None
            }
        })
        .last()
        .unwrap()
}

#[test]
fn genesis_config_works() {
    new_test_ext().execute_with(|| {
        assert_eq!(Treasury::pot(), 0);
        assert_eq!(Treasury::proposal_count(), 0);
    });
}

fn tip_hash() -> H256 {
    BlakeTwo256::hash_of(&(BlakeTwo256::hash(b"awesome.dot"), 3u128))
}

fn tip_hash_error() -> H256 {
    BlakeTwo256::hash_of(&(BlakeTwo256::hash(b"awesome.dot.ksm"), 3u128))
}

#[test]
fn tip_new_cannot_be_used_twice() {
    new_test_ext().execute_with(|| {
        Balances::make_free_balance_be(&Treasury::account_id(), 101);
        assert_ok!(Tips::tip_new(
            Origin::signed(10),
            b"awesome.dot".to_vec(),
            3,
            10
        ));
        assert_noop!(
            Tips::tip_new(Origin::signed(11), b"awesome.dot".to_vec(), 3, 10),
            Error::<Test>::AlreadyKnown
        );
    });
}

#[test]
fn credit_tip_new_and_credit_tip() {
    new_test_ext().execute_with(|| {
        assert_ok!(DeeperNode::im_online(Origin::signed(3)));

        let credit_data_before = CreditData {
            campaign_id: 0,
            credit: 100,
            initial_credit_level: CreditLevel::One,
            rank_in_initial_credit_level: 0,
            number_of_referees: 0,
            current_credit_level: CreditLevel::One,
            reward_eras: 0,
        };
        assert_ok!(Credit::add_or_update_credit_data(
            RawOrigin::Root.into(),
            3,
            credit_data_before.clone()
        ));
        assert_eq!(Credit::user_credit(3), Some(credit_data_before));

        Balances::make_free_balance_be(&Treasury::account_id(), 101);
        // Invalid Tippers will not commit tips
        assert_noop!(
            Tips::credit_tip_new(Origin::signed(9), b"awesome.dot".to_vec(), 3, 10),
            BadOrigin
        );

        assert_ok!(Tips::credit_tip_new(
            Origin::signed(10),
            b"awesome.dot".to_vec(),
            3,
            10
        ));
        assert_noop!(
            Tips::credit_tip_new(Origin::signed(11), b"awesome.dot".to_vec(), 3, 10),
            Error::<Test>::AlreadyKnown
        );

        let h = tip_hash();
        let e = tip_hash_error();
        assert_ok!(Tips::credit_tip(Origin::signed(10), h.clone(), 10));
        assert_ok!(Tips::credit_tip(Origin::signed(11), h.clone(), 10));
        // UnknownTip
        assert_noop!(
            Tips::credit_tip(Origin::signed(12), e.clone(), 10),
            Error::<Test>::UnknownTip
        );
        assert_ok!(Tips::credit_tip(Origin::signed(12), h.clone(), 10));
        assert_noop!(Tips::tip(Origin::signed(9), h.clone(), 10), BadOrigin);
        System::set_block_number(2);
        // UnknownTip
        assert_noop!(
            Tips::close_credit_tip(Origin::signed(100), e.clone()),
            Error::<Test>::UnknownTip
        );

        assert_ok!(Tips::close_credit_tip(Origin::signed(100), h.into()));

        let credit_data_after = CreditData {
            campaign_id: 0,
            credit: 110,
            initial_credit_level: CreditLevel::One,
            rank_in_initial_credit_level: 0,
            number_of_referees: 0,
            current_credit_level: CreditLevel::One,
            reward_eras: 0,
        };

        assert_eq!(Credit::user_credit(3), Some(credit_data_after));
    });
}

#[test]
fn credit_tip_new_and_check_get_level() {
    new_test_ext().execute_with(|| {
        assert_ok!(DeeperNode::im_online(Origin::signed(3)));

        let credit_data_before = CreditData {
            campaign_id: 0,
            credit: 195,
            initial_credit_level: CreditLevel::One,
            rank_in_initial_credit_level: 0,
            number_of_referees: 0,
            current_credit_level: CreditLevel::One,
            reward_eras: 0,
        };
        assert_ok!(Credit::add_or_update_credit_data(
            RawOrigin::Root.into(),
            3,
            credit_data_before.clone()
        ));
        assert_eq!(Credit::user_credit(3), Some(credit_data_before));

        assert_ok!(Tips::credit_tip_new(
            Origin::signed(10),
            b"awesome.dot".to_vec(),
            3,
            10
        ));
        let h = tip_hash();
        assert_ok!(Tips::credit_tip(Origin::signed(10), h.clone(), 10));
        assert_ok!(Tips::credit_tip(Origin::signed(11), h.clone(), 10));
        assert_ok!(Tips::credit_tip(Origin::signed(12), h.clone(), 10));
        // Premature
        assert_noop!(
            Tips::close_credit_tip(Origin::signed(100), h.clone()),
            Error::<Test>::Premature
        );
        System::set_block_number(2);
        assert_ok!(Tips::close_credit_tip(Origin::signed(100), h.into()));

        // UnknownTip
        assert_noop!(
            Tips::close_credit_tip(Origin::signed(100), h.clone()),
            Error::<Test>::UnknownTip
        );

        let credit_data_after = CreditData {
            campaign_id: 0,
            credit: 205,
            initial_credit_level: CreditLevel::One,
            rank_in_initial_credit_level: 0,
            number_of_referees: 0,
            current_credit_level: CreditLevel::Two,
            reward_eras: 0,
        };

        assert_eq!(Credit::user_credit(3), Some(credit_data_after));
    });
}

#[test]
fn report_awesome_and_tip_works() {
    new_test_ext().execute_with(|| {
        Balances::make_free_balance_be(&Treasury::account_id(), 101);
        assert_ok!(Balances::set_balance(RawOrigin::Root.into(), 5, 100, 0));
        TipPaymentAddress::<Test>::put(5);

        assert_ok!(Tips::report_awesome(
            Origin::signed(0),
            b"awesome.dot".to_vec(),
            3
        ));
        assert_eq!(Balances::reserved_balance(0), 12);
        assert_eq!(Balances::free_balance(0), 88);

        // other reports don't count.
        assert_noop!(
            Tips::report_awesome(Origin::signed(1), b"awesome.dot".to_vec(), 3),
            Error::<Test>::AlreadyKnown
        );

        let h = tip_hash();
        assert_ok!(Tips::tip(Origin::signed(10), h.clone(), 10));
        assert_ok!(Tips::tip(Origin::signed(11), h.clone(), 10));
        assert_ok!(Tips::tip(Origin::signed(12), h.clone(), 10));
        assert_noop!(Tips::tip(Origin::signed(9), h.clone(), 10), BadOrigin);
        System::set_block_number(2);
        assert_ok!(Tips::close_tip(Origin::signed(100), h.into()));
        assert_eq!(Balances::reserved_balance(0), 0);
        assert_eq!(Balances::free_balance(0), 102);
        assert_eq!(Balances::free_balance(3), 8);
    });
}

#[test]
fn report_awesome_from_beneficiary_and_tip_works() {
    new_test_ext().execute_with(|| {
        Balances::make_free_balance_be(&Treasury::account_id(), 101);
        assert_ok!(Balances::set_balance(RawOrigin::Root.into(), 5, 100, 0));
        TipPaymentAddress::<Test>::put(5);
        assert_ok!(Tips::report_awesome(
            Origin::signed(0),
            b"awesome.dot".to_vec(),
            0
        ));
        assert_eq!(Balances::reserved_balance(0), 12);
        assert_eq!(Balances::free_balance(0), 88);
        let h = BlakeTwo256::hash_of(&(BlakeTwo256::hash(b"awesome.dot"), 0u128));
        assert_ok!(Tips::tip(Origin::signed(10), h.clone(), 10));
        assert_ok!(Tips::tip(Origin::signed(11), h.clone(), 10));
        assert_ok!(Tips::tip(Origin::signed(12), h.clone(), 10));
        System::set_block_number(2);
        assert_ok!(Tips::close_tip(Origin::signed(100), h.into()));
        assert_eq!(Balances::reserved_balance(0), 0);
        assert_eq!(Balances::free_balance(0), 110);
    });
}

#[test]
fn close_tip_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        Balances::make_free_balance_be(&Treasury::account_id(), 101);
        assert_eq!(Treasury::pot(), 100);

        assert_ok!(Tips::tip_new(
            Origin::signed(10),
            b"awesome.dot".to_vec(),
            3,
            10
        ));

        let h = tip_hash();

        assert_eq!(last_event(), TipEvent::NewTip(h));

        assert_ok!(Tips::tip(Origin::signed(11), h.clone(), 10));

        assert_noop!(
            Tips::close_tip(Origin::signed(0), h.into()),
            Error::<Test>::StillOpen
        );

        assert_ok!(Tips::tip(Origin::signed(12), h.clone(), 10));

        assert_eq!(last_event(), TipEvent::TipClosing(h));

        assert_noop!(
            Tips::close_tip(Origin::signed(0), h.into()),
            Error::<Test>::Premature
        );

        System::set_block_number(2);
        assert_noop!(Tips::close_tip(Origin::none(), h.into()), BadOrigin);
        assert_ok!(Tips::close_tip(Origin::signed(0), h.into()));
        assert_eq!(Balances::free_balance(3), 10);

        assert_eq!(last_event(), TipEvent::TipClosed(h, 3, 10));

        assert_noop!(
            Tips::close_tip(Origin::signed(100), h.into()),
            Error::<Test>::UnknownTip
        );
    });
}

#[test]
fn slash_tip_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        Balances::make_free_balance_be(&Treasury::account_id(), 101);
        assert_eq!(Treasury::pot(), 100);

        assert_eq!(Balances::reserved_balance(0), 0);
        assert_eq!(Balances::free_balance(0), 100);

        assert_ok!(Tips::report_awesome(
            Origin::signed(0),
            b"awesome.dot".to_vec(),
            3
        ));

        assert_eq!(Balances::reserved_balance(0), 12);
        assert_eq!(Balances::free_balance(0), 88);

        let h = tip_hash();
        assert_eq!(last_event(), TipEvent::NewTip(h));

        // can't remove from any origin
        assert_noop!(Tips::slash_tip(Origin::signed(0), h.clone()), BadOrigin);

        // can remove from root.
        assert_ok!(Tips::slash_tip(Origin::root(), h.clone()));
        assert_eq!(last_event(), TipEvent::TipSlashed(h, 0, 12));

        // tipper slashed
        assert_eq!(Balances::reserved_balance(0), 0);
        assert_eq!(Balances::free_balance(0), 88);
    });
}

#[test]
fn retract_tip_works() {
    new_test_ext().execute_with(|| {
        // with report awesome
        Balances::make_free_balance_be(&Treasury::account_id(), 101);
        assert_ok!(Tips::report_awesome(
            Origin::signed(0),
            b"awesome.dot".to_vec(),
            3
        ));
        let h = tip_hash();
        assert_ok!(Tips::tip(Origin::signed(10), h.clone(), 10));
        assert_ok!(Tips::tip(Origin::signed(11), h.clone(), 10));
        assert_ok!(Tips::tip(Origin::signed(12), h.clone(), 10));
        assert_noop!(
            Tips::retract_tip(Origin::signed(10), h.clone()),
            Error::<Test>::NotFinder
        );
        assert_ok!(Tips::retract_tip(Origin::signed(0), h.clone()));
        System::set_block_number(2);
        assert_noop!(
            Tips::close_tip(Origin::signed(0), h.into()),
            Error::<Test>::UnknownTip
        );

        // with tip new
        Balances::make_free_balance_be(&Treasury::account_id(), 101);
        assert_ok!(Tips::tip_new(
            Origin::signed(10),
            b"awesome.dot".to_vec(),
            3,
            10
        ));
        let h = tip_hash();
        assert_ok!(Tips::tip(Origin::signed(11), h.clone(), 10));
        assert_ok!(Tips::tip(Origin::signed(12), h.clone(), 10));
        assert_noop!(
            Tips::retract_tip(Origin::signed(0), h.clone()),
            Error::<Test>::NotFinder
        );
        assert_ok!(Tips::retract_tip(Origin::signed(10), h.clone()));
        System::set_block_number(2);
        assert_noop!(
            Tips::close_tip(Origin::signed(10), h.into()),
            Error::<Test>::UnknownTip
        );
    });
}

#[test]
fn tip_median_calculation_works() {
    new_test_ext().execute_with(|| {
        Balances::make_free_balance_be(&Treasury::account_id(), 101);
        assert_ok!(Tips::tip_new(
            Origin::signed(10),
            b"awesome.dot".to_vec(),
            3,
            0
        ));
        let h = tip_hash();
        assert_ok!(Tips::tip(Origin::signed(11), h.clone(), 10));
        assert_ok!(Tips::tip(Origin::signed(12), h.clone(), 1000000));
        System::set_block_number(2);
        assert_ok!(Tips::close_tip(Origin::signed(0), h.into()));
        assert_eq!(Balances::free_balance(3), 10);
    });
}

#[test]
fn tip_changing_works() {
    new_test_ext().execute_with(|| {
        Balances::make_free_balance_be(&Treasury::account_id(), 101);
        assert_ok!(Tips::tip_new(
            Origin::signed(10),
            b"awesome.dot".to_vec(),
            3,
            10000
        ));
        let h = tip_hash();
        assert_ok!(Tips::tip(Origin::signed(11), h.clone(), 10000));
        assert_ok!(Tips::tip(Origin::signed(12), h.clone(), 10000));
        assert_ok!(Tips::tip(Origin::signed(13), h.clone(), 0));
        assert_ok!(Tips::tip(Origin::signed(14), h.clone(), 0));
        assert_ok!(Tips::tip(Origin::signed(12), h.clone(), 1000));
        assert_ok!(Tips::tip(Origin::signed(11), h.clone(), 100));
        assert_ok!(Tips::tip(Origin::signed(10), h.clone(), 10));
        System::set_block_number(2);
        assert_ok!(Tips::close_tip(Origin::signed(0), h.into()));
        assert_eq!(Balances::free_balance(3), 10);
    });
}

#[test]
fn test_last_reward_migration() {
    let mut s = Storage::default();

    #[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
    pub struct OldOpenTip<
        AccountId: Parameter,
        Balance: Parameter,
        BlockNumber: Parameter,
        Hash: Parameter,
    > {
        /// The hash of the reason for the tip. The reason should be a human-readable UTF-8 encoded
        /// string. A URL would be sensible.
        reason: Hash,
        /// The account to be tipped.
        who: AccountId,
        /// The account who began this tip and the amount held on deposit.
        finder: Option<(AccountId, Balance)>,
        /// The block number at which this tip will close if `Some`. If `None`, then no closing is
        /// scheduled.
        closes: Option<BlockNumber>,
        /// The members who have voted for this tip. Sorted by AccountId.
        tips: Vec<(AccountId, Balance)>,
    }

    let reason1 = BlakeTwo256::hash(b"reason1");
    let hash1 = BlakeTwo256::hash_of(&(reason1, 10u64));

    let old_tip_finder = OldOpenTip::<u128, u64, u64, H256> {
        reason: reason1,
        who: 10,
        finder: Some((20, 30)),
        closes: Some(13),
        tips: vec![(40, 50), (60, 70)],
    };

    let reason2 = BlakeTwo256::hash(b"reason2");
    let hash2 = BlakeTwo256::hash_of(&(reason2, 20u64));

    let old_tip_no_finder = OldOpenTip::<u128, u64, u64, H256> {
        reason: reason2,
        who: 20,
        finder: None,
        closes: Some(13),
        tips: vec![(40, 50), (60, 70)],
    };

    let data = vec![
        (
            pallet_tips::Tips::<Test>::hashed_key_for(hash1),
            old_tip_finder.encode().to_vec(),
        ),
        (
            pallet_tips::Tips::<Test>::hashed_key_for(hash2),
            old_tip_no_finder.encode().to_vec(),
        ),
    ];

    s.top = data.into_iter().collect();

    sp_io::TestExternalities::new(s).execute_with(|| {
        let module = pallet_tips::Tips::<Test>::module_prefix();
        let item = pallet_tips::Tips::<Test>::storage_prefix();
        Tips::migrate_retract_tip_for_tip_new(module, item);

        // Test w/ finder
        assert_eq!(
            pallet_tips::Tips::<Test>::get(hash1),
            Some(OpenTip {
                reason: reason1,
                who: 10,
                finder: 20,
                deposit: 30,
                closes: Some(13),
                tips: vec![(40, 50), (60, 70)],
                finders_fee: true,
                credits: vec![],
            })
        );

        // Test w/o finder
        assert_eq!(
            pallet_tips::Tips::<Test>::get(hash2),
            Some(OpenTip {
                reason: reason2,
                who: 20,
                finder: Default::default(),
                deposit: 0,
                closes: Some(13),
                tips: vec![(40, 50), (60, 70)],
                finders_fee: false,
                credits: vec![],
            })
        );
    });
}

#[test]
fn genesis_funding_works() {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    let initial_funding = 100;
    pallet_balances::GenesisConfig::<Test> {
        // Total issuance will be 200 with treasury account initialized with 100.
        balances: vec![(0, 100), (Treasury::account_id(), initial_funding)],
    }
    .assimilate_storage(&mut t)
    .unwrap();
    GenesisBuild::<Test>::assimilate_storage(&pallet_treasury::GenesisConfig, &mut t).unwrap();
    let mut t: sp_io::TestExternalities = t.into();

    t.execute_with(|| {
        assert_eq!(
            Balances::free_balance(Treasury::account_id()),
            initial_funding
        );
        assert_eq!(
            Treasury::pot(),
            initial_funding - Balances::minimum_balance()
        );
    });
}
