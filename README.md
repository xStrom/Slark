# Slark

Slark is a tool for viewing static and animated images.

```sh
cargo run /path/to/image.gif
```

You can load multiple images into Slark and then drag them around the canvas.

```sh
cargo run /path/to/image.gif /and/another.webp third.jpg
```

Use PGUP / PGDN to control their Z-ordering. Mouse wheel to zoom. DEL to remove an image.

Ctrl+S / Ctrl+O to save / open a project file which remembers all the opened images and their location, z-order, and zoom level.

Supported image formats are GIF, WebP, JPEG, and PNG.

## Project status

Slark is in early development. There are plenty of bugs and development time is limited.

## License

© Copyright 2019-2022 [Kaur Kuut](https://www.kaurkuut.com/)

Slark is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.