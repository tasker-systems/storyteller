// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(
            &[
                "../../proto/storyteller/v1/engine.proto",
                "../../proto/storyteller/v1/composer.proto",
            ],
            &["../../proto"],
        )?;
    Ok(())
}
