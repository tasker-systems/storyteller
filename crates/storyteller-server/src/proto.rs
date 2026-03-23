// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

//! Generated protobuf types and gRPC service definitions.

pub mod storyteller {
    pub mod v1 {
        tonic::include_proto!("storyteller.v1");
    }
}

pub use storyteller::v1::*;
