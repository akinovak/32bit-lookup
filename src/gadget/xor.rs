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
pub  struct ChunkWord {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl ChunkWord {
    pub fn new(x: u8, y: u8, z: u8) -> Self {
        ChunkWord {
            x,
            y,
            z
        }
    }
}

/// A variable stored in advice columns corresponding to a row of [`SpreadTableConfig`].
#[derive(Clone, Debug)]
pub struct ChunkVar {
    pub x: Option<Chunk>,
    pub y: Option<Chunk>,
    pub z: Option<Chunk>,
}

#[derive(Clone, Debug)]
pub struct Inputs {
    pub x: Column<Advice>,
    pub y: Column<Advice>,
    pub z: Column<Advice>,
}

#[derive(Clone, Debug)]
pub struct Table {
    pub x: TableColumn,
    pub y: TableColumn,
    pub z: TableColumn,
}

#[derive(Clone, Debug)]
pub struct TableConfig {
    pub input: Inputs,
    pub table: Table,
}

#[derive(Clone, Debug)]
pub struct TableChip<F: FieldExt> {
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

    pub fn add_row(
        &self,
        region: &mut Region<'_, F>,
        row: usize,
        x: Option<Chunk>,
        y: Option<Chunk>,
        z: Option<Chunk>
    ) -> Result<(), Error> {
        let config = self.config();

        region.assign_advice(
            || format!("x: {}", row), 
            config.input.x, 
            row, 
            || { 
                x.map(|x| F::from(*x as u64))
                .ok_or(Error::Synthesis)
            }
        )?;

        region.assign_advice(
            || format!("y: {}", row), 
            config.input.y, 
            row, 
            || { 
                y.map(|y| F::from(*y as u64))
                .ok_or(Error::Synthesis)
            }
        )?;

        region.assign_advice(
            || format!("z: {}", row), 
            config.input.z, 
            row, 
            || { 
                z.map(|z| F::from(*z as u64))
                .ok_or(Error::Synthesis)
            }
        )?;

        Ok(())
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

    use crate::word::{Chunk};

    use pasta_curves::pallas;

    #[test]
    fn lookup_table() {
        #[derive(Copy, Clone, Debug)]
        struct MyCircuit {}

        impl Circuit<pallas::Base> for MyCircuit {
            type Config = TableConfig;
            type FloorPlanner = SimpleFloorPlanner;

            fn without_witnesses(&self) -> Self {
                MyCircuit {}
            }

            fn configure(meta: &mut ConstraintSystem<pallas::Base>) -> Self::Config {
                let input_x = meta.advice_column();
                let input_y = meta.advice_column();
                let input_z = meta.advice_column();

                TableChip::configure(meta, input_x, input_y, input_z)
            }

            fn synthesize(
                &self,
                config: Self::Config,
                mut layouter: impl Layouter<pallas::Base>,
            ) -> Result<(), Error> {


                TableChip::load(config.clone(), &mut layouter)?;

                let table_chip = TableChip::construct(config);

                layouter.assign_region(
                    || "compress",
                    |mut region| {
                        table_chip.add_row(
                            &mut region, 
                            0, 
                            Some(Chunk::new(0)), 
                            Some(Chunk::new(1)), 
                            Some(Chunk::new(1))
                        )?;

                        table_chip.add_row(
                            &mut region, 
                            1, 
                            Some(Chunk::new(0b00110011)), 
                            Some(Chunk::new(0b00110011)), 
                            Some(Chunk::new(0b00000000))
                        )?;

                        table_chip.add_row(
                            &mut region, 
                            2, 
                            Some(Chunk::new(0b01010101)), 
                            Some(Chunk::new(0b10101010)), 
                            Some(Chunk::new(0b11111111))
                        )?;
                        Ok(())
                    },
                )?;

                Ok(())
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
