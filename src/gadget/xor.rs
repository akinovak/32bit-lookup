use halo2::{
    arithmetic::FieldExt,
    circuit::{Chip, Layouter, Region},
    pasta::pallas,
    plonk::{Advice, Column, ConstraintSystem, Error, TableColumn},
    poly::Rotation,
};
use std::convert::TryInto;
use std::marker::PhantomData;

use crate::word::{Chunk, AssignedChunk};

const XOR_BITS: usize = 8;

/// An input word into a lookup, containing (tag, dense, spread)
#[derive(Copy, Clone, Debug)]
pub(super) struct ChunkWord {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl ChunkWord {
    pub(super) fn new(x: u8, y: u8, z: u8) -> Self {
        ChunkWord {
            x,
            y,
            z
        }
    }
}

/// A variable stored in advice columns corresponding to a row of [`SpreadTableConfig`].
#[derive(Clone, Debug)]
pub(super) struct ChunkVar {
    pub x: Option<Chunk>,
    pub y: Option<Chunk>,
    pub z: Option<Chunk>,
}

// impl ChunkVar {
//     pub(super) fn with_lookup(
//         region: &mut Region<'_, pallas::Base>,
//         cols: &SpreadInputs,
//         row: usize,
//         x: Option<Chunk>,
//         y: Option<Chunk>,
//         z: Option<Chunk>,
//     ) -> Result<Self, Error> {
//         // let tag = word.map(|word| word.tag);
//         // let dense_val = word.map(|word| word.dense);
//         // let spread_val = word.map(|word| word.spread);

//         // region.assign_advice(
//         //     || "x",
//         //     cols.tag,
//         //     row,
//         //     || {
//         //         tag.map(|tag| pallas::Base::from(tag as u64))
//         //             .ok_or(Error::Synthesis)
//         //     },
//         // )?;

//         // let dense =
//         //     AssignedBits::<DENSE>::assign_bits(region, || "dense", cols.dense, row, dense_val)?;

//         // let spread =
//         //     AssignedBits::<SPREAD>::assign_bits(region, || "spread", cols.spread, row, spread_val)?;

//         // let x = AssignedChunk::assign_chunk()

//         // Ok(ChunkVar { tag, dense, spread })
//     }
// }

#[derive(Clone, Debug)]
pub(super) struct Inputs {
    pub(super) x: Column<Advice>,
    pub(super) y: Column<Advice>,
    pub(super) z: Column<Advice>,
}

#[derive(Clone, Debug)]
pub(super) struct Table {
    pub(super) x: TableColumn,
    pub(super) y: TableColumn,
    pub(super) z: TableColumn,
}

#[derive(Clone, Debug)]
pub(super) struct TableConfig {
    pub input: Inputs,
    pub table: Table,
}

#[derive(Clone, Debug)]
pub(super) struct TableChip<F: FieldExt> {
    config: TableConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> Chip<F> for TableChip<F> {
    type Config = TableConfig;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

impl<F: FieldExt> TableChip<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        input_x: Column<Advice>,
        input_y: Column<Advice>,
        input_z: Column<Advice>,
    ) -> <Self as Chip<F>>::Config {
        let table_x = meta.lookup_table_column();
        let table_y = meta.lookup_table_column();
        let table_z = meta.lookup_table_column();

        meta.lookup(|meta| {
            let x_cur = meta.query_advice(input_x, Rotation::cur());
            let y_cur = meta.query_advice(input_y, Rotation::cur());
            let z_cur = meta.query_advice(input_z, Rotation::cur());

            vec![
                (x_cur, table_x),
                (y_cur, table_y),
                (z_cur, table_z),
            ]
        });

        TableConfig {
            input: Inputs {
                x: input_x,
                y: input_y,
                z: input_z,
            },
            table: Table {
                x: table_x,
                y: table_y,
                z: table_z,
            },
        }
    }

    pub fn construct(config: TableConfig) -> Self {
        TableChip {
            config, 
            _marker: PhantomData
        }
    }

    pub fn load(
        config: TableConfig,
        layouter: &mut impl Layouter<F>,
    ) -> Result<<Self as Chip<F>>::Loaded, Error> {
        let config = config.clone();
        layouter.assign_table(
            || "xor table",
            |mut table| {
                // We generate the row values lazily (we only need them during keygen).
                // let mut rows = SpreadTableConfig::generate::<F>();

                let mut row_offset = 0;
                for l in 0..1 << XOR_BITS {
                    for r in 0..1 << XOR_BITS {
                        table.assign_cell(
                            || format!("xor_l_col row {}", row_offset),
                            config.table.x,
                            row_offset,
                            || Ok(F::from(l)),
                        )?;
                        table.assign_cell(
                            || format!("xor_r_col row {}", row_offset),
                            config.table.y,
                            row_offset,
                            || Ok(F::from(r)),
                        )?;
                        table.assign_cell(
                            || format!("xor_o_col row {}", row_offset),
                            config.table.z,
                            row_offset,
                            || Ok(F::from(l ^ r)),
                        )?;
                        row_offset += 1;
                    }
                }
                Ok(())
            },
        )
    }
}


#[cfg(test)]
mod tests {
    use super::{TableChip, TableConfig};
    use rand::Rng;

    use halo2::{
        arithmetic::FieldExt,
        circuit::{Layouter, SimpleFloorPlanner},
        dev::MockProver,
        pasta::Fp,
        plonk::{Advice, Circuit, Column, ConstraintSystem, Error},
    };

    #[test]
    fn lookup_table() {
        /// This represents an advice column at a certain row in the ConstraintSystem
        #[derive(Copy, Clone, Debug)]
        pub struct Variable(Column<Advice>, usize);

        struct MyCircuit {}

        impl<F: FieldExt> Circuit<F> for MyCircuit {
            type Config = TableConfig;
            type FloorPlanner = SimpleFloorPlanner;

            fn without_witnesses(&self) -> Self {
                MyCircuit {}
            }

            fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
                let input_x = meta.advice_column();
                let input_y = meta.advice_column();
                let input_z = meta.advice_column();

                TableChip::configure(meta, input_x, input_y, input_z)
            }

            fn synthesize(
                &self,
                config: Self::Config,
                mut layouter: impl Layouter<F>,
            ) -> Result<(), Error> {
                TableChip::load(config.clone(), &mut layouter)?;

                layouter.assign_region(
                    || "xor_test",
                    |mut gate| {
                        let mut row = 0;
                        let mut add_row = |x, y, z| -> Result<(), Error> {
                            gate.assign_advice(|| "z", config.input.x, row, || Ok(x))?;
                            gate.assign_advice(|| "y", config.input.y, row, || Ok(y))?;
                            gate.assign_advice(
                                || "z",
                                config.input.z,
                                row,
                                || Ok(z),
                            )?;
                            row += 1;
                            Ok(())
                        };

                        // Test the first few small values.
                        add_row(F::zero(), F::one(), F::one())?;
                        // add_row(F::zero(), F::from(0b001), F::from(0b000001))?;
                        // add_row(F::zero(), F::from(0b010), F::from(0b000100))?;
                        // add_row(F::zero(), F::from(0b011), F::from(0b000101))?;
                        // add_row(F::zero(), F::from(0b100), F::from(0b010000))?;
                        // add_row(F::zero(), F::from(0b101), F::from(0b010001))?;

                        // Test the tag boundaries:
                        // 7-bit
                        // add_row(F::zero(), F::from(0b1111111), F::from(0b01010101010101))?;
                        // add_row(F::one(), F::from(0b10000000), F::from(0b0100000000000000))?;
                        // // - 10-bit
                        // add_row(
                        //     F::one(),
                        //     F::from(0b1111111111),
                        //     F::from(0b01010101010101010101),
                        // )?;
                        // add_row(
                        //     F::from(2),
                        //     F::from(0b10000000000),
                        //     F::from(0b0100000000000000000000),
                        // )?;
                        // // - 11-bit
                        // add_row(
                        //     F::from(2),
                        //     F::from(0b11111111111),
                        //     F::from(0b0101010101010101010101),
                        // )?;
                        // add_row(
                        //     F::from(3),
                        //     F::from(0b100000000000),
                        //     F::from(0b010000000000000000000000),
                        // )?;
                        // // - 13-bit
                        // add_row(
                        //     F::from(3),
                        //     F::from(0b1111111111111),
                        //     F::from(0b01010101010101010101010101),
                        // )?;
                        // add_row(
                        //     F::from(4),
                        //     F::from(0b10000000000000),
                        //     F::from(0b0100000000000000000000000000),
                        // )?;
                        // // - 14-bit
                        // add_row(
                        //     F::from(4),
                        //     F::from(0b11111111111111),
                        //     F::from(0b0101010101010101010101010101),
                        // )?;
                        // add_row(
                        //     F::from(5),
                        //     F::from(0b100000000000000),
                        //     F::from(0b010000000000000000000000000000),
                        // )?;

                        // Test random lookup values
                        // let mut rng = rand::thread_rng();

                        // fn interleave_u16_with_zeros(word: u16) -> u32 {
                        //     let mut word: u32 = word.into();
                        //     word = (word ^ (word << 8)) & 0x00ff00ff;
                        //     word = (word ^ (word << 4)) & 0x0f0f0f0f;
                        //     word = (word ^ (word << 2)) & 0x33333333;
                        //     word = (word ^ (word << 1)) & 0x55555555;
                        //     word
                        // }

                        // for _ in 0..10 {
                        //     let word: u16 = rng.gen();
                        //     add_row(
                        //         F::from(u64::from(get_tag(word))),
                        //         F::from(u64::from(word)),
                        //         F::from(u64::from(interleave_u16_with_zeros(word))),
                        //     )?;
                        // }

                        Ok(())
                    },
                )
            }
        }

        let circuit: MyCircuit = MyCircuit {};

        let prover = match MockProver::<Fp>::run(17, &circuit, vec![]) {
            Ok(prover) => prover,
            Err(e) => panic!("{:?}", e),
        };
        assert_eq!(prover.verify(), Ok(()));
    }
}
