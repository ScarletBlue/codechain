// Copyright 2018 Kodebox, Inc.
// This file is part of CodeChain.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::BTreeMap;
use std::fmt;

use cjson;
use ctypes::Address;

use super::pod_account::PodAccount;

/// State of all accounts in the system expressed in Plain Old Data.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PodState(BTreeMap<Address, PodAccount>);

impl PodState {
    /// Get the underlying map.
    pub fn get(&self) -> &BTreeMap<Address, PodAccount> {
        &self.0
    }
}

impl From<cjson::spec::State> for PodState {
    fn from(s: cjson::spec::State) -> PodState {
        let state: BTreeMap<_, _> =
            s.into_iter().filter(|(_, acc)| !acc.is_empty()).map(|(addr, acc)| (addr.into(), acc.into())).collect();
        PodState(state)
    }
}

impl fmt::Display for PodState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (add, acc) in &self.0 {
            writeln!(f, "{} => {}", add, acc)?;
        }
        Ok(())
    }
}
