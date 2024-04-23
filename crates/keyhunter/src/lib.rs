/// Copyright © 2024 Don Isaac
///
/// This file is part of KeyHunter.
///
/// KeyHunter is free software: you can redistribute it and/or modify it
/// under the terms of the GNU General Public License as published by the Free
/// Software Foundation, either version 3 of the License, or (at your option)
/// any later version.
///
/// KeyHunter is distributed in the hope that it will be useful, but WITHOUT
/// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
/// FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for
/// more details.
///
/// You should have received a copy of the GNU General Public License along with
/// KeyHunter. If not, see <https://www.gnu.org/licenses/>.
pub mod config;
mod extract;
pub(crate) mod http;
pub mod report;
mod walk;

pub use config::Config;
pub use extract::{
    ApiKeyCollector, ApiKeyError, ApiKeyExtractor, ApiKeyMessage, ApiKeyReceiver, ApiKeySender,
};
pub use walk::{ScriptMessage, ScriptReceiver, WebsiteWalker};
