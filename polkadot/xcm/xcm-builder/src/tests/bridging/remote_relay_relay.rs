// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! This test is when we're sending an XCM from a parachain whose relay-chain hosts a bridge to
//! another relay-chain. The destination of the XCM is within the global consensus of the
//! remote side of the bridge.

use super::*;

parameter_types! {
	pub UniversalLocation: Junctions = [GlobalConsensus(Local::get()), Parachain(1000)].into();
	pub RelayUniversalLocation: Junctions = [GlobalConsensus(Local::get())].into();
	pub RemoteUniversalLocation: Junctions = [GlobalConsensus(Remote::get())].into();
	pub RemoteNetwork: Location = AncestorThen(1, GlobalConsensus(Remote::get())).into();
	pub BridgeTable: Vec<NetworkExportTableItem> = vec![
		NetworkExportTableItem::new(
			Remote::get(),
			None,
			Location::parent(),
			None
		)
	];
}
type TheBridge =
	TestBridge<BridgeBlobDispatcher<TestRemoteIncomingRouter, RemoteUniversalLocation, ()>>;
type RelayExporter = HaulBlobExporter<TheBridge, RemoteNetwork, AlwaysLatest, ()>;
type LocalInnerRouter =
	UnpaidExecutingRouter<UniversalLocation, RelayUniversalLocation, RelayExporter>;
type LocalBridgeRouter =
	UnpaidRemoteExporter<NetworkExportTable<BridgeTable>, LocalInnerRouter, UniversalLocation>;
type LocalRouter = TestTopic<(LocalInnerRouter, LocalBridgeRouter)>;

/// ```nocompile
///  local                                  |                                      remote
///                                         |
///     GlobalConsensus(Local::get())   ========>    GlobalConsensus(Remote::get())
///            /\                           |
///            ||                           |
///            ||                           |
///            ||                           |
///     Parachain(1000)                     |
/// ```
#[test]
fn sending_to_bridged_chain_works() {
	maybe_with_topic(|| {
		let msg = Xcm(vec![Trap(1)]);
		assert_eq!(
			send_xcm::<LocalRouter>((Parent, Parent, Remote::get()).into(), msg).unwrap().1,
			Assets::new()
		);
		assert_eq!(TheBridge::service(), 1);
		assert_eq!(
			take_received_remote_messages(),
			vec![(
				Here.into(),
				xcm_with_topic(
					[0; 32],
					vec![
						UniversalOrigin(Local::get().into()),
						DescendOrigin(Parachain(1000).into()),
						Trap(1)
					]
				)
			)]
		);
		let entry = LogEntry {
			local: UniversalLocation::get(),
			remote: RelayUniversalLocation::get(),
			id: maybe_forward_id_for(&[0; 32]),
			message: xcm_with_topic(
				maybe_forward_id_for(&[0; 32]),
				vec![
					UnpaidExecution { weight_limit: Unlimited, check_origin: None },
					ExportMessage {
						network: ByGenesis([1; 32]),
						destination: Here,
						xcm: xcm_with_topic([0; 32], vec![Trap(1)]),
					},
				],
			),
			outcome: Outcome::Complete { used: test_weight(2) },
			paid: false,
		};
		assert_eq!(RoutingLog::take(), vec![entry]);
	});
}

/// ```nocompile
///  local                                  |                                      remote
///                                         |
///     GlobalConsensus(Local::get())   ========>    GlobalConsensus(Remote::get())
///            /\                           |                             ||
///            ||                           |                             ||
///            ||                           |                             ||
///            ||                           |                             \/
///     Parachain(1000)                     |                       Parachain(1000)
/// ```
#[test]
fn sending_to_parachain_of_bridged_chain_works() {
	maybe_with_topic(|| {
		let msg = Xcm(vec![Trap(1)]);
		let dest = (Parent, Parent, Remote::get(), Parachain(1000)).into();
		assert_eq!(send_xcm::<LocalRouter>(dest, msg).unwrap().1, Assets::new());
		assert_eq!(TheBridge::service(), 1);
		let expected = vec![(
			Parachain(1000).into(),
			xcm_with_topic(
				[0; 32],
				vec![
					UniversalOrigin(Local::get().into()),
					DescendOrigin(Parachain(1000).into()),
					Trap(1),
				],
			),
		)];
		assert_eq!(take_received_remote_messages(), expected);
		let entry = LogEntry {
			local: UniversalLocation::get(),
			remote: RelayUniversalLocation::get(),
			id: maybe_forward_id_for(&[0; 32]),
			message: xcm_with_topic(
				maybe_forward_id_for(&[0; 32]),
				vec![
					UnpaidExecution { weight_limit: Unlimited, check_origin: None },
					ExportMessage {
						network: ByGenesis([1; 32]),
						destination: Parachain(1000).into(),
						xcm: xcm_with_topic([0; 32], vec![Trap(1)]),
					},
				],
			),
			outcome: Outcome::Complete { used: test_weight(2) },
			paid: false,
		};
		assert_eq!(RoutingLog::take(), vec![entry]);
	})
}
