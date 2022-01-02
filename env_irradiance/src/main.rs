use anyhow::Result;
use image::{io::Reader as ImageReader, ImageBuffer, Rgba};

const PI: f32 = 3.141593;

#[derive(Default)]
struct IrradianceCoefficients {
    data: [[f32; 3]; 9],
}

impl IrradianceCoefficients {
    fn from_cubemap(faces: &[ImageBuffer<Rgba<u8>, Vec<u8>>]) -> Self {
        // Integrate over environment and compute coefficients
        // http://graphics.stanford.edu/papers/envmap/envmap.pdf

        let mut coeffs = IrradianceCoefficients::default();
        let size = (faces[0].width(), faces[1].height());

        println!("computing coefficients, face_size: {:?}", size);

        for (i, face) in faces.iter().enumerate() {
            println!("integrating face {}", i);
            for col in 0..size.0 {
                for row in 0..size.1 {
                    let cube_coords = CubeCoords {
                        u: col as f32 / size.0 as f32,
                        v: row as f32 / size.1 as f32,
                        index: i,
                    };

                    let cartesian = cube_coords.to_cartesian(size);
                    let (x, y, z) = (cartesian[0], cartesian[1], cartesian[2]);
                    let spherical = SphereCoords::from(cartesian);
                    let d_omega = (2.0 * PI / size.0 as f32)
                        * (2.0 * PI / size.0 as f32)
                        * sinc(spherical.theta);

                    let hdr = cube_coords.sample(size, &faces);

                    for col in 0..3 {
                        let mut c = 0.0;

                        /* L_{00}.  Note that Y_{00} = 0.282095 */
                        c = 0.282095;
                        coeffs.data[0][col] += hdr[col] * c * d_omega;

                        /* L_{1m}. -1 <= m <= 1.  The linear terms */
                        c = 0.488603;
                        coeffs.data[1][col] += hdr[col] * (c * y) * d_omega; /* Y_{1-1} = 0.488603 y  */
                        coeffs.data[2][col] += hdr[col] * (c * z) * d_omega; /* Y_{10}  = 0.488603 z  */
                        coeffs.data[3][col] += hdr[col] * (c * x) * d_omega; /* Y_{11}  = 0.488603 x  */

                        /* The Quadratic terms, L_{2m} -2 <= m <= 2 */

                        /* First, L_{2-2}, L_{2-1}, L_{21} corresponding to xy,yz,xz */
                        c = 1.092548;
                        coeffs.data[4][col] += hdr[col] * (c * x * y) * d_omega; /* Y_{2-2} = 1.092548 xy */
                        coeffs.data[5][col] += hdr[col] * (c * y * z) * d_omega; /* Y_{2-1} = 1.092548 yz */
                        coeffs.data[7][col] += hdr[col] * (c * x * z) * d_omega; /* Y_{21}  = 1.092548 xz */

                        /* L_{20}.  Note that Y_{20} = 0.315392 (3z^2 - 1) */
                        c = 0.315392;
                        coeffs.data[6][col] += hdr[col] * (c * (3.0 * z * z - 1.0)) * d_omega;

                        /* L_{22}.  Note that Y_{22} = 0.546274 (x^2 - y^2) */
                        c = 0.546274;
                        coeffs.data[8][col] += hdr[col] * (c * (x * x - y * y)) * d_omega;
                    }
                }
            }
        }
        coeffs
    }

    // Outputs three 4x4 quadratic matrices
    fn to_matrices(&self) -> [[[f32; 4]; 4]; 3] {
        let mut matrix: [[[f32; 4]; 4]; 3] = Default::default();
        let c1 = 0.429043;
        let c2 = 0.511664;
        let c3 = 0.743125;
        let c4 = 0.886227;
        let c5 = 0.247708;

        for col in 0..3 {
            matrix[0][0][col] = c1 * self.data[8][col]; /* c1 L_{22}  */
            matrix[0][1][col] = c1 * self.data[4][col]; /* c1 L_{2-2} */
            matrix[0][2][col] = c1 * self.data[7][col]; /* c1 L_{21}  */
            matrix[0][3][col] = c2 * self.data[3][col]; /* c2 L_{11}  */

            matrix[1][0][col] = c1 * self.data[4][col]; /* c1 L_{2-2} */
            matrix[1][1][col] = -c1 * self.data[8][col]; /*-c1 L_{22}  */
            matrix[1][2][col] = c1 * self.data[5][col]; /* c1 L_{2-1} */
            matrix[1][3][col] = c2 * self.data[1][col]; /* c2 L_{1-1} */

            matrix[2][0][col] = c1 * self.data[7][col]; /* c1 L_{21}  */
            matrix[2][1][col] = c1 * self.data[5][col]; /* c1 L_{2-1} */
            matrix[2][2][col] = c3 * self.data[6][col]; /* c3 L_{20}  */
            matrix[2][3][col] = c2 * self.data[2][col]; /* c2 L_{10}  */

            matrix[3][0][col] = c2 * self.data[3][col]; /* c2 L_{11}  */
            matrix[3][1][col] = c2 * self.data[1][col]; /* c2 L_{1-1} */
            matrix[3][2][col] = c2 * self.data[2][col]; /* c2 L_{10}  */

            /* c4 L_{00} - c5 L_{20} */
            matrix[3][3][col] = c4 * self.data[0][col] - c5 * self.data[6][col];
        }

        matrix
    }

    fn output(&self) {
        println!(
            "L_0,0: ({}, {}, {})",
            self.data[0][0], self.data[0][1], self.data[0][2]
        );
        println!(
            "L_1,-1: ({}, {}, {})",
            self.data[1][0], self.data[1][1], self.data[1][2]
        );
        println!(
            "L_1,0: ({}, {}, {})",
            self.data[2][0], self.data[2][1], self.data[2][2]
        );
        println!(
            "L_1,1: ({}, {}, {})",
            self.data[3][0], self.data[3][1], self.data[3][2]
        );
        println!(
            "L_2,-2: ({}, {}, {})",
            self.data[4][0], self.data[4][1], self.data[4][2]
        );
        println!(
            "L_2,-1: ({}, {}, {})",
            self.data[5][0], self.data[5][1], self.data[5][2]
        );
        println!(
            "L_2,0: ({}, {}, {})",
            self.data[6][0], self.data[6][1], self.data[6][2]
        );
        println!(
            "L_2,1: ({}, {}, {})",
            self.data[7][0], self.data[7][1], self.data[7][2]
        );
        println!(
            "L_2,2: ({}, {}, {})",
            self.data[8][0], self.data[8][1], self.data[8][2]
        );
    }
}

fn main() -> Result<()> {
    // Load environment map

    let path = "./engine/src/sources/static/cubemaps/default_lowres";
    let file_ext = "png";
    let dirs = vec!["px", "nx", "py", "ny", "pz", "nz"];

    let faces: Vec<ImageBuffer<Rgba<u8>, Vec<u8>>> = dirs
        .iter()
        .map(|dir| {
            let img_path = format!("{}/{}.{}", path, dir, file_ext);
            println!("loading cubemap at {}", img_path);
            image::io::Reader::open(img_path)
                .unwrap()
                .decode()
                .unwrap()
                .into_rgba8()
        })
        .collect();

    let coefficients = IrradianceCoefficients::from_cubemap(&faces);
    coefficients.output();

    Ok(())
}

struct SphereCoords {
    r: f32,
    theta: f32,
    phi: f32,
}

impl From<[f32; 3]> for SphereCoords {
    fn from(cartesian: [f32; 3]) -> Self {
        Self {
            r: (cartesian[0].powi(2) + cartesian[1].powi(2) + cartesian[2].powi(2)).sqrt(),
            theta: cartesian[1].atan2(cartesian[0]),
            phi: (cartesian[0].powi(2) + cartesian[1].powi(2))
                .sqrt()
                .atan2(cartesian[2]),
        }
    }
}

struct CubeCoords {
    index: usize,
    u: f32,
    v: f32,
}

impl CubeCoords {
    fn sample(&self, size: (u32, u32), faces: &[ImageBuffer<Rgba<u8>, Vec<u8>>]) -> [f32; 3] {
        let face = &faces[self.index];
        let pixel = face
            .get_pixel(
                (self.u * size.0 as f32) as u32,
                (self.v * size.1 as f32) as u32,
            )
            .0;
        [
            pixel[0] as f32 / 255.0,
            pixel[1] as f32 / 255.0,
            pixel[2] as f32 / 255.0,
        ]
    }

    // https://en.wikipedia.org/wiki/Cube_mapping#Memory_addressing
    fn to_cartesian(&self, size: (u32, u32)) -> [f32; 3] {
        let uc = 2.0 * self.u - 1.0;
        let vc = 2.0 * self.v - 1.0;
        match self.index {
            0 => [1.0, vc, -uc],  // POSITIVE X
            1 => [-1.0, vc, uc],  // NEGATIVE X
            2 => [uc, 1.0, -vc],  // POSITIVE Y
            3 => [uc, -1.0, vc],  // NEGATIVE Y
            4 => [uc, vc, 1.0],   // POSITIVE Z
            5 => [-uc, vc, -1.0], // NEGATIVE Z
            _ => panic!("cubemap cannot have more than 6 faces"),
        }
    }
}

// https://en.wikipedia.org/wiki/Cube_mapping#Memory_addressing
impl From<[f32; 3]> for CubeCoords {
    fn from(cartesian: [f32; 3]) -> Self {
        let abs_x = cartesian[0].abs();
        let abs_y = cartesian[1].abs();
        let abs_z = cartesian[2].abs();

        let is_x_pos = cartesian[0] > 0.0;
        let is_y_pos = cartesian[1] > 0.0;
        let is_z_pos = cartesian[2] > 0.0;

        let mut max_axis = 0.0f32;
        let mut uc = 0.0f32;
        let mut vc = 0.0f32;

        let mut index = 0usize;

        if is_x_pos && abs_x >= abs_y && abs_x >= abs_z {
            // u (0 to 1) goes from +z to -z
            // v (0 to 1) goes from -y to +y
            max_axis = abs_x;
            uc = -cartesian[2];
            vc = cartesian[1];
            index = 0;
        }

        // NEGATIVE X
        if !is_x_pos && abs_x >= abs_y && abs_x >= abs_z {
            // u (0 to 1) goes from -z to +z
            // v (0 to 1) goes from -y to +y
            max_axis = abs_x;
            uc = cartesian[2];
            vc = cartesian[1];
            index = 1;
        }

        // POSITIVE Y
        if is_y_pos && abs_y >= abs_x && abs_y >= abs_z {
            // u (0 to 1) goes from -x to +x
            // v (0 to 1) goes from +z to -z
            max_axis = abs_y;
            uc = cartesian[0];
            vc = -cartesian[2];
            index = 2;
        }

        // NEGATIVE Y
        if !is_y_pos && abs_y >= abs_x && abs_y >= abs_z {
            // u (0 to 1) goes from -x to +x
            // v (0 to 1) goes from -z to +z
            max_axis = abs_y;
            uc = cartesian[0];
            vc = cartesian[2];
            index = 3;
        }

        // POSITIVE Z
        if is_z_pos && abs_z >= abs_x && abs_z >= abs_y {
            // u (0 to 1) goes from -x to +x
            // v (0 to 1) goes from -y to +y
            max_axis = abs_z;
            uc = cartesian[0];
            vc = cartesian[1];
            index = 4;
        }

        // NEGATIVE Z
        if !is_z_pos && abs_z >= abs_x && abs_z >= abs_y {
            // u (0 to 1) goes from +x to -x
            // v (0 to 1) goes from -y to +y
            max_axis = abs_z;
            uc = -cartesian[0];
            vc = cartesian[1];
            index = 5;
        }

        let u = 0.5 * (uc / max_axis + 1.0);
        let v = 0.5 * (vc / max_axis + 1.0);

        CubeCoords { u, v, index }
    }
}

fn sinc(x: f32) -> f32 {
    if x.abs() < 1.0 * 10.0_f32.powf(-4.0) {
        1.0
    } else {
        x.sin() / x
    }
}
