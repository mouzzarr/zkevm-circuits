use halo2_proofs::{
    circuit::{AssignedCell, Cell, Region, RegionIndex, Value},
    halo2curves::bn256::Fr,
    plonk::Error,
};
use zkevm_circuits::util::Challenges;

use crate::{constants::LOG_DEGREE, util::assert_equal};

use super::RlcConfig;

impl RlcConfig {
    /// initialize the chip with fixed 0 and 1 cells
    pub(crate) fn init(&self, region: &mut Region<Fr>) -> Result<(), Error> {
        region.assign_fixed(|| "const zero", self.fixed, 0, || Value::known(Fr::zero()))?;
        region.assign_fixed(|| "const one", self.fixed, 1, || Value::known(Fr::one()))?;
        region.assign_fixed(|| "const two", self.fixed, 2, || Value::known(Fr::from(2)))?;
        region.assign_fixed(|| "const four", self.fixed, 3, || Value::known(Fr::from(4)))?;
        region.assign_fixed(
            || "const eight",
            self.fixed,
            4,
            || Value::known(Fr::from(8)),
        )?;
        region.assign_fixed(
            || "const thirty two",
            self.fixed,
            5,
            || Value::known(Fr::from(32)),
        )?;
        Ok(())
    }

    #[inline]
    pub(crate) fn zero_cell(&self, region_index: RegionIndex) -> Cell {
        Cell {
            region_index,
            row_offset: 0,
            column: self.fixed.into(),
        }
    }

    #[inline]
    pub(crate) fn one_cell(&self, region_index: RegionIndex) -> Cell {
        Cell {
            region_index,
            row_offset: 1,
            column: self.fixed.into(),
        }
    }

    #[inline]
    pub(crate) fn two_cell(&self, region_index: RegionIndex) -> Cell {
        Cell {
            region_index,
            row_offset: 2,
            column: self.fixed.into(),
        }
    }

    #[inline]
    pub(crate) fn four_cell(&self, region_index: RegionIndex) -> Cell {
        Cell {
            region_index,
            row_offset: 3,
            column: self.fixed.into(),
        }
    }

    #[inline]
    pub(crate) fn eight_cell(&self, region_index: RegionIndex) -> Cell {
        Cell {
            region_index,
            row_offset: 4,
            column: self.fixed.into(),
        }
    }

    #[inline]
    pub(crate) fn thirty_two_cell(&self, region_index: RegionIndex) -> Cell {
        Cell {
            region_index,
            row_offset: 5,
            column: self.fixed.into(),
        }
    }

    pub(crate) fn load_private(
        &self,
        region: &mut Region<Fr>,
        f: &Fr,
        offset: &mut usize,
    ) -> Result<AssignedCell<Fr, Fr>, Error> {
        let res = region.assign_advice(
            || "load private",
            self.phase_2_column,
            *offset,
            || Value::known(*f),
        );
        *offset += 1;
        res
    }

    pub(crate) fn read_challenge(
        &self,
        region: &mut Region<Fr>,
        challenge_value: Challenges<Value<Fr>>,
        offset: &mut usize,
    ) -> Result<AssignedCell<Fr, Fr>, Error> {
        let challenge_value = challenge_value.keccak_input();
        let challenge_cell = region.assign_advice(
            || "assign challenge",
            self.phase_2_column,
            *offset,
            || challenge_value,
        )?;
        self.enable_challenge.enable(region, *offset)?;
        *offset += 1;
        Ok(challenge_cell)
    }

    /// Enforce the element in f is a zero element.
    pub(crate) fn enforce_zero(
        &self,
        region: &mut Region<Fr>,
        f: &AssignedCell<Fr, Fr>,
    ) -> Result<(), Error> {
        let zero_cell = self.zero_cell(f.cell().region_index);
        region.constrain_equal(f.cell(), zero_cell)
    }

    /// Enforce the element in f is a binary element.
    pub(crate) fn enforce_binary(
        &self,
        region: &mut Region<Fr>,
        f: &AssignedCell<Fr, Fr>,
        offset: &mut usize,
    ) -> Result<(), Error> {
        let f2 = self.mul(region, f, f, offset)?;
        region.constrain_equal(f.cell(), f2.cell())
    }

    /// Enforce res = a + b
    #[allow(dead_code)]
    pub(crate) fn add(
        &self,
        region: &mut Region<Fr>,
        a: &AssignedCell<Fr, Fr>,
        b: &AssignedCell<Fr, Fr>,
        offset: &mut usize,
    ) -> Result<AssignedCell<Fr, Fr>, Error> {
        self.selector.enable(region, *offset)?;
        let one_cell = self.one_cell(a.cell().region_index);

        a.copy_advice(|| "a", region, self.phase_2_column, *offset)?;
        let one = region.assign_advice(
            || "c",
            self.phase_2_column,
            *offset + 1,
            || Value::known(Fr::one()),
        )?;
        region.constrain_equal(one.cell(), one_cell)?;
        b.copy_advice(|| "c", region, self.phase_2_column, *offset + 2)?;
        let d = region.assign_advice(
            || "d",
            self.phase_2_column,
            *offset + 3,
            || a.value() + b.value(),
        )?;
        *offset += 4;

        Ok(d)
    }

    /// Enforce res = a - b
    pub(crate) fn sub(
        &self,
        region: &mut Region<Fr>,
        a: &AssignedCell<Fr, Fr>,
        b: &AssignedCell<Fr, Fr>,
        offset: &mut usize,
    ) -> Result<AssignedCell<Fr, Fr>, Error> {
        self.selector.enable(region, *offset)?;
        let one_cell = self.one_cell(a.cell().region_index);

        let res = region.assign_advice(
            || "a",
            self.phase_2_column,
            *offset,
            || a.value() - b.value(),
        )?;
        let one = region.assign_advice(
            || "b",
            self.phase_2_column,
            *offset + 1,
            || Value::known(Fr::one()),
        )?;
        region.constrain_equal(one.cell(), one_cell)?;
        b.copy_advice(|| "c", region, self.phase_2_column, *offset + 2)?;
        a.copy_advice(|| "d", region, self.phase_2_column, *offset + 3)?;
        *offset += 4;

        Ok(res)
    }

    /// Enforce res = a * b
    pub(crate) fn mul(
        &self,
        region: &mut Region<Fr>,
        a: &AssignedCell<Fr, Fr>,
        b: &AssignedCell<Fr, Fr>,
        offset: &mut usize,
    ) -> Result<AssignedCell<Fr, Fr>, Error> {
        self.selector.enable(region, *offset)?;
        let zero_cell = self.zero_cell(a.cell().region_index);

        a.copy_advice(|| "a", region, self.phase_2_column, *offset)?;
        b.copy_advice(|| "b", region, self.phase_2_column, *offset + 1)?;
        let zero = region.assign_advice(
            || "b",
            self.phase_2_column,
            *offset + 2,
            || Value::known(Fr::zero()),
        )?;
        region.constrain_equal(zero.cell(), zero_cell)?;
        let d = region.assign_advice(
            || "d",
            self.phase_2_column,
            *offset + 3,
            || a.value() * b.value(),
        )?;
        *offset += 4;

        Ok(d)
    }

    /// Enforce res = a * b + c
    pub(crate) fn mul_add(
        &self,
        region: &mut Region<Fr>,
        a: &AssignedCell<Fr, Fr>,
        b: &AssignedCell<Fr, Fr>,
        c: &AssignedCell<Fr, Fr>,
        offset: &mut usize,
    ) -> Result<AssignedCell<Fr, Fr>, Error> {
        self.selector.enable(region, *offset)?;

        a.copy_advice(|| "a", region, self.phase_2_column, *offset)?;
        b.copy_advice(|| "b", region, self.phase_2_column, *offset + 1)?;
        c.copy_advice(|| "c", region, self.phase_2_column, *offset + 2)?;
        let d = region.assign_advice(
            || "d",
            self.phase_2_column,
            *offset + 3,
            || a.value() * b.value() + c.value(),
        )?;
        *offset += 4;

        Ok(d)
    }

    /// caller need to ensure a is binary
    /// return !a
    pub(crate) fn not(
        &self,
        region: &mut Region<Fr>,
        a: &AssignedCell<Fr, Fr>,
        offset: &mut usize,
    ) -> Result<AssignedCell<Fr, Fr>, Error> {
        let one_cell = self.one_cell(a.cell().region_index);
        let one = self.load_private(region, &Fr::one(), offset)?;
        region.constrain_equal(one_cell, one.cell())?;
        self.sub(region, &one, a, offset)
    }

    // if cond = 1 return a, else b
    pub(crate) fn select(
        &self,
        region: &mut Region<Fr>,
        a: &AssignedCell<Fr, Fr>,
        b: &AssignedCell<Fr, Fr>,
        cond: &AssignedCell<Fr, Fr>,
        offset: &mut usize,
    ) -> Result<AssignedCell<Fr, Fr>, Error> {
        // (cond - 1) * b + cond * a
        let cond_not = self.not(region, cond, offset)?;
        let tmp = self.mul(region, a, cond, offset)?;
        self.mul_add(region, b, &cond_not, &tmp, offset)
    }

    // Returns inputs[0] + challenge * inputs[1] + ... + challenge^k * inputs[k]
    #[allow(dead_code)]
    pub(crate) fn rlc(
        &self,
        region: &mut Region<Fr>,
        inputs: &[AssignedCell<Fr, Fr>],
        challenge: &AssignedCell<Fr, Fr>,
        offset: &mut usize,
    ) -> Result<AssignedCell<Fr, Fr>, Error> {
        let mut acc = inputs[0].clone();
        for input in inputs.iter().skip(1) {
            acc = self.mul_add(region, &acc, challenge, input, offset)?;
        }
        Ok(acc)
    }

    // Returns inputs[0] + challenge * inputs[1] + ... + challenge^k * inputs[k]
    pub(crate) fn rlc_with_flag(
        &self,
        region: &mut Region<Fr>,
        inputs: &[AssignedCell<Fr, Fr>],
        challenge: &AssignedCell<Fr, Fr>,
        flags: &[AssignedCell<Fr, Fr>],
        offset: &mut usize,
    ) -> Result<AssignedCell<Fr, Fr>, Error> {
        assert!(flags.len() == inputs.len());

        let mut acc = inputs[0].clone();
        for (input, flag) in inputs.iter().zip(flags.iter()).skip(1) {
            let tmp = self.mul_add(region, &acc, challenge, input, offset)?;
            acc = self.select(region, &tmp, &acc, flag, offset)?;
        }
        Ok(acc)
    }

    // padded the columns
    #[allow(dead_code)]
    pub(crate) fn pad(&self, region: &mut Region<Fr>, offset: &usize) -> Result<(), Error> {
        for index in *offset..(1 << LOG_DEGREE) - 1 {
            region.assign_advice(
                || "pad",
                self.phase_2_column,
                index,
                || Value::known(Fr::zero()),
            )?;
        }
        Ok(())
    }

    // decompose a field element into 254 bits of boolean cells
    pub(crate) fn decomposition(
        &self,
        region: &mut Region<Fr>,
        input: &AssignedCell<Fr, Fr>,
        offset: &mut usize,
    ) -> Result<Vec<AssignedCell<Fr, Fr>>, Error> {
        let mut input_element = Fr::default();
        input.value().map(|&x| input_element = x);

        let bits = input_element
            .to_bytes()
            .iter()
            .flat_map(byte_to_bits_le)
            .collect::<Vec<_>>();
        // sanity check
        {
            let mut reconstructed = Fr::zero();
            bits.iter().rev().for_each(|bit| {
                reconstructed *= Fr::from(2);
                reconstructed += Fr::from(*bit as u64);
            });
            assert_eq!(reconstructed, input_element);
        }

        let bit_cells = bits
            .iter()
            .take(254) // hard coded for BN curve
            .map(|&bit| {
                let cell = self.load_private(region, &Fr::from(bit as u64), offset)?;
                self.enforce_binary(region, &cell, offset)?;
                Ok(cell)
            })
            .collect::<Result<Vec<_>, Error>>()?;

        let mut acc = {
            let zero = self.load_private(region, &Fr::from(0), offset)?;
            let zero_cell = self.zero_cell(zero.cell().region_index);
            region.constrain_equal(zero_cell, zero.cell())?;
            zero
        };

        let two = {
            let two = self.load_private(region, &Fr::from(2), offset)?;
            let two_cell = self.two_cell(two.cell().region_index);
            region.constrain_equal(two_cell, two.cell())?;
            two
        };

        for bit in bit_cells.iter().rev() {
            acc = self.mul_add(region, &acc, &two, bit, offset)?;
        }

        // sanity check
        assert_equal(&acc, input);

        region.constrain_equal(acc.cell(), input.cell())?;

        Ok(bit_cells)
    }

    // return a boolean if a is smaller than b
    // requires that both a and b are smallish
    pub(crate) fn is_smaller_than(
        &self,
        region: &mut Region<Fr>,
        a: &AssignedCell<Fr, Fr>,
        b: &AssignedCell<Fr, Fr>,
        offset: &mut usize,
    ) -> Result<AssignedCell<Fr, Fr>, Error> {
        // when a and b are both small (as in our use case)
        // if a < b, (a-b) will under flow and the highest bit of (a-b) be one
        // else,  the highest bit of (a-b) be zero
        let sub = self.sub(region, a, b, offset)?;
        let bits = self.decomposition(region, &sub, offset)?;
        Ok(bits[253].clone())
    }
}
#[inline]
fn byte_to_bits_le(byte: &u8) -> Vec<u8> {
    let mut res = vec![];
    let mut t = *byte;
    for _ in 0..8 {
        res.push(t & 1);
        t >>= 1;
    }
    res
}
