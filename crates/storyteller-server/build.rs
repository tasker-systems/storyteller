// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true) // client stubs for integration tests
        .compile_protos(
            &[
                "../../proto/storyteller/v1/engine.proto",
                "../../proto/storyteller/v1/composer.proto",
            ],
            &["../../proto"],
        )?;
    Ok(())
}
