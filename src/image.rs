// Copyright 2022 the Slark Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

use imgref::ImgVec;
use rgb::RGBA8;

/// Represents a single frame of a potentially multi-frame image.
pub struct Frame {
    pub image: ImgVec<RGBA8>,
    pub delay: i64,
}
