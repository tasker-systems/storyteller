// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! RabbitMQ integration for distributed messaging.
//!
//! See: `docs/technical/infrastructure-architecture.md`, `docs/technical/event-system.md`
//!
//! RabbitMQ is used for tasker-core workflow dispatch (deferred event processing).
//! NOT used for in-process turn-cycle events — those use Bevy events.

pub mod tasker;
