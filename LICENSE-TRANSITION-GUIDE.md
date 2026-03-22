# License Transition Guide: MIT → AGPL-3.0 + Commercial

## Files to Change

### 1. Replace `LICENSE`

Replace the current MIT license text with the full AGPL-3.0 text. The easiest way:

```bash
# From the storyteller repo root
curl -o LICENSE https://www.gnu.org/licenses/agpl-3.0.txt
```

Or use GitHub's license picker: edit the LICENSE file on GitHub, click "Choose a license template," select "GNU Affero General Public License v3.0," and commit.

GitHub will automatically detect the AGPL-3.0 from this file and display it on the repo page.

### 2. Add `LICENSING.md`

Place the provided `LICENSING.md` at the repo root. This is the human-readable explanation of the dual-license structure that the README will point to.

### 3. Add `CLA.md`

Place the provided `CLA.md` at the repo root. This defines the contributor license agreement.

### 4. Update `README.md`

Replace the current license section:

```markdown
## License

See [LICENSE](LICENSE).
```

With:

```markdown
## License

Storyteller is dual-licensed under the [GNU Affero General Public License v3.0](LICENSE) and a commercial license. See [LICENSING.md](LICENSING.md) for full details.

Contributions are accepted under the [Contributor License Agreement](CLA.md).
```

### 5. Update `Cargo.toml`

Add workspace-level metadata to the root `Cargo.toml`:

```toml
[workspace.package]
license = "AGPL-3.0-only"
```

Individual crate `Cargo.toml` files can inherit this:

```toml
[package]
license.workspace = true
```

The SPDX identifier `AGPL-3.0-only` (as opposed to `AGPL-3.0-or-later`) specifies version 3 exactly, with no automatic upgrade to future versions. This is the safer choice — it preserves your ability to evaluate future AGPL versions before adopting them.

### 6. Add Source File Headers

Add to all `.rs` files:

```rust
// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Tasker Systems. All rights reserved.
// See LICENSING.md for details.
```

Add to all `.py` files:

```python
# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (c) 2026 Tasker Systems. All rights reserved.
# See LICENSING.md for details.
```

A one-time script can add these to existing files. For new files, include them as part of the project's coding conventions (document in CLAUDE.md and any contributor guide).

### 7. Optional: GitHub PR Template

Create `.github/PULL_REQUEST_TEMPLATE.md`:

```markdown
## Description

<!-- Describe your changes -->

## CLA

- [ ] I have read and agree to the [Contributor License Agreement](CLA.md).
```

### 8. Optional: CLA Bot

For automated CLA enforcement, consider [CLA Assistant](https://github.com/cla-assistant/cla-assistant) — a GitHub App that checks CLA signatures on PRs. This is overkill for now but useful if the project attracts regular external contributors.

## What About `storyteller-data`?

The `storyteller-data` repo should have its own license that reflects its private, non-open-source status. A simple approach:

```
Copyright (c) 2026 Tasker Systems. All rights reserved.

This repository and its contents are proprietary. No license is granted
for use, modification, or distribution without explicit written permission
from Tasker Systems.
```

This makes the boundary between the open engine and the private data explicit in both repos.

## Transition Checklist

- [ ] Replace LICENSE with AGPL-3.0 text
- [ ] Add LICENSING.md
- [ ] Add CLA.md
- [ ] Update README.md license section
- [ ] Add `license = "AGPL-3.0-only"` to workspace Cargo.toml
- [ ] Add SPDX headers to source files
- [ ] Add PR template with CLA checkbox
- [ ] Add proprietary license to storyteller-data
- [ ] Update CLAUDE.md to mention SPDX headers for new files
- [ ] Create `licensing@tasker-systems.com` email alias (or decide on contact method)
- [ ] Commit with a clear message: "Relicense from MIT to AGPL-3.0-only with dual-license structure"
