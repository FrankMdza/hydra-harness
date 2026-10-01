//! `sorted`, as CPython's `list.sort` does it.
//!
//! `queue` sorts tasks by a key that holds a score, and a score can be NaN. `x < nan` and
//! `nan < x` are both false, so the comparison is not an order and the result depends on
//! which pairs the algorithm compares. A stable sort from another library gives a different,
//! equally defensible answer. To print what Python prints this is a port of the algorithm in
//! CPython 3.14's `Objects/listobject.c`: natural runs, binary insertion up to `minrun`,
//! powersort's merge order, and merges that gallop. Names follow the C so the two can be read
//! side by side. `tests/differential.rs` checks it against `python3` on lists full of NaN.

const MIN_GALLOP: usize = 7;
const MAX_MINRUN: usize = 64;

struct Run {
    base: usize,
    len: usize,
    power: u32,
}

struct MergeState<F> {
    /// The list being sorted: indices into the caller's items.
    a: Vec<usize>,
    less: F,
    min_gallop: usize,
    pending: Vec<Run>,
}

/// The order `sorted(range(n), key=...)` returns, given `less(i, j)` for "item `i` sorts
/// before item `j`". An error from `less` stops the sort, as an exception does.
pub fn sorted<E>(
    n: usize,
    less: impl FnMut(usize, usize) -> Result<bool, E>,
) -> Result<Vec<usize>, E> {
    let mut ms = MergeState {
        a: (0..n).collect(),
        less,
        min_gallop: MIN_GALLOP,
        pending: Vec::new(),
    };
    if n < 2 {
        return Ok(ms.a);
    }
    let minrun = merge_compute_minrun(n);
    let mut lo = 0;
    let mut nremaining = n;
    while nremaining > 0 {
        let mut run = ms.count_run(lo, nremaining)?;
        if run < minrun {
            let force = nremaining.min(minrun);
            ms.binarysort(lo, force, run)?;
            run = force;
        }
        ms.found_new_run(run, n)?;
        ms.pending.push(Run {
            base: lo,
            len: run,
            power: 0,
        });
        lo += run;
        nremaining -= run;
    }
    ms.merge_force_collapse()?;
    Ok(ms.a)
}

/// The run length short runs are extended to: `n` itself below 64, else a number from 32 to
/// 64 such that `n / minrun` is a power of two or just under one.
fn merge_compute_minrun(mut n: usize) -> usize {
    let mut r = 0;
    while n >= MAX_MINRUN {
        r |= n & 1;
        n >>= 1;
    }
    n + r
}

/// Where `key` belongs in the sorted `a`, left of any equal elements. `hint` is where to
/// start looking; the search doubles its step away from there, then bisects.
fn gallop_left<E>(
    less: &mut impl FnMut(usize, usize) -> Result<bool, E>,
    key: usize,
    a: &[usize],
    hint: usize,
) -> Result<usize, E> {
    let n = a.len();
    let mut lastofs: usize = 0;
    let mut ofs: usize = 1;
    let (mut lo, mut hi);
    if less(a[hint], key)? {
        // a[hint] < key: gallop right until a[hint + lastofs] < key <= a[hint + ofs].
        let maxofs = n - hint;
        while ofs < maxofs {
            if less(a[hint + ofs], key)? {
                lastofs = ofs;
                ofs = (ofs << 1) + 1;
            } else {
                break;
            }
        }
        ofs = ofs.min(maxofs);
        lo = (lastofs + hint) as isize;
        hi = hint + ofs;
    } else {
        // key <= a[hint]: gallop left until a[hint - ofs] < key <= a[hint - lastofs].
        let maxofs = hint + 1;
        while ofs < maxofs {
            if less(a[hint - ofs], key)? {
                break;
            }
            lastofs = ofs;
            ofs = (ofs << 1) + 1;
        }
        ofs = ofs.min(maxofs);
        lo = hint as isize - ofs as isize;
        hi = hint - lastofs;
    }
    lo += 1;
    let mut lo = lo as usize;
    while lo < hi {
        let m = lo + ((hi - lo) >> 1);
        if less(a[m], key)? {
            lo = m + 1;
        } else {
            hi = m;
        }
    }
    Ok(hi)
}

/// As `gallop_left`, but right of any equal elements.
fn gallop_right<E>(
    less: &mut impl FnMut(usize, usize) -> Result<bool, E>,
    key: usize,
    a: &[usize],
    hint: usize,
) -> Result<usize, E> {
    let n = a.len();
    let mut lastofs: usize = 0;
    let mut ofs: usize = 1;
    let (mut lo, mut hi);
    if less(key, a[hint])? {
        // key < a[hint]: gallop left until a[hint - ofs] <= key < a[hint - lastofs].
        let maxofs = hint + 1;
        while ofs < maxofs {
            if less(key, a[hint - ofs])? {
                lastofs = ofs;
                ofs = (ofs << 1) + 1;
            } else {
                break;
            }
        }
        ofs = ofs.min(maxofs);
        lo = hint as isize - ofs as isize;
        hi = hint - lastofs;
    } else {
        // a[hint] <= key: gallop right until a[hint + lastofs] <= key < a[hint + ofs].
        let maxofs = n - hint;
        while ofs < maxofs {
            if less(key, a[hint + ofs])? {
                break;
            }
            lastofs = ofs;
            ofs = (ofs << 1) + 1;
        }
        ofs = ofs.min(maxofs);
        lo = (lastofs + hint) as isize;
        hi = hint + ofs;
    }
    lo += 1;
    let mut lo = lo as usize;
    while lo < hi {
        let m = lo + ((hi - lo) >> 1);
        if less(key, a[m])? {
            hi = m;
        } else {
            lo = m + 1;
        }
    }
    Ok(hi)
}

/// How `merge_lo` and `merge_hi` leave their loops, for the C's `goto` labels.
enum Exit {
    Succeed,
    /// `CopyB` in `merge_lo`, `CopyA` in `merge_hi`: one element of the copied run is left
    /// and it belongs at the far end.
    CopyRest,
}

impl<F, E> MergeState<F>
where
    F: FnMut(usize, usize) -> Result<bool, E>,
{
    /// The length of the run starting at `lo`: ascending, or strictly descending and then
    /// reversed in place, with runs of equal elements inside it kept in their order.
    fn count_run(&mut self, lo: usize, nremaining: usize) -> Result<usize, E> {
        let mut n = 1;
        while n < nremaining {
            if (self.less)(self.a[lo + n], self.a[lo + n - 1])? {
                break;
            }
            n += 1;
        }
        if n == nremaining {
            return Ok(n);
        }
        // a[lo + n] is strictly less. With n > 1 that ends an ascending run, unless the run
        // is all equal elements, which is a descending run too.
        if n > 1 {
            if (self.less)(self.a[lo], self.a[lo + n - 1])? {
                return Ok(n);
            }
            self.a[lo..lo + n].reverse();
        }
        n += 1;
        let mut neq = 0;
        while n < nremaining {
            if (self.less)(self.a[lo + n], self.a[lo + n - 1])? {
                if neq > 0 {
                    self.a[lo + n - neq - 1..lo + n].reverse();
                    neq = 0;
                }
            } else if (self.less)(self.a[lo + n - 1], self.a[lo + n])? {
                break;
            } else {
                neq += 1;
            }
            n += 1;
        }
        if neq > 0 {
            self.a[lo + n - neq - 1..lo + n].reverse();
        }
        self.a[lo..lo + n].reverse();
        while n < nremaining {
            if (self.less)(self.a[lo + n], self.a[lo + n - 1])? {
                break;
            }
            n += 1;
        }
        Ok(n)
    }

    /// Binary insertion sort of `a[lo..lo + n]`, whose first `ok` elements are sorted.
    fn binarysort(&mut self, lo: usize, n: usize, ok: usize) -> Result<(), E> {
        for ok in ok.max(1)..n {
            let pivot = self.a[lo + ok];
            let (mut left, mut right) = (0, ok);
            while left < right {
                let mid = (left + right) >> 1;
                if (self.less)(pivot, self.a[lo + mid])? {
                    right = mid;
                } else {
                    left = mid + 1;
                }
            }
            self.a.copy_within(lo + left..lo + ok, lo + left + 1);
            self.a[lo + left] = pivot;
        }
        Ok(())
    }

    /// Powersort: a new run of `n2` elements was found; merge the pending runs whose "power"
    /// is higher than the boundary between the last run and the new one.
    fn found_new_run(&mut self, n2: usize, listlen: usize) -> Result<(), E> {
        let Some(last) = self.pending.last() else {
            return Ok(());
        };
        let power = powerloop(last.base, last.len, n2, listlen);
        while self.pending.len() > 1 && self.pending[self.pending.len() - 2].power > power {
            self.merge_at(self.pending.len() - 2)?;
        }
        if let Some(last) = self.pending.last_mut() {
            last.power = power;
        }
        Ok(())
    }

    fn merge_force_collapse(&mut self) -> Result<(), E> {
        while self.pending.len() > 1 {
            let mut n = self.pending.len() - 2;
            if n > 0 && self.pending[n - 1].len < self.pending[n + 1].len {
                n -= 1;
            }
            self.merge_at(n)?;
        }
        Ok(())
    }

    /// Merge the pending runs `i` and `i + 1`.
    fn merge_at(&mut self, i: usize) -> Result<(), E> {
        let (mut ssa, mut na) = (self.pending[i].base, self.pending[i].len);
        let (ssb, nb) = (self.pending[i + 1].base, self.pending[i + 1].len);
        self.pending[i].len = na + nb;
        self.pending.remove(i + 1);
        // Where does b start in a? Elements of a before that are already in place.
        let k = gallop_right(&mut self.less, self.a[ssb], &self.a[ssa..ssa + na], 0)?;
        ssa += k;
        na -= k;
        if na == 0 {
            return Ok(());
        }
        // Where does a end in b? Elements of b after that are already in place.
        let nb = gallop_left(
            &mut self.less,
            self.a[ssa + na - 1],
            &self.a[ssb..ssb + nb],
            nb - 1,
        )?;
        if nb == 0 {
            return Ok(());
        }
        if na <= nb {
            self.merge_lo(ssa, na, ssb, nb)
        } else {
            self.merge_hi(ssa, na, ssb, nb)
        }
    }

    /// Merge `a[ssa..ssa + na]` with the run after it, `nb` long, left to right, with the
    /// first run copied aside. `na <= nb`.
    fn merge_lo(&mut self, ssa: usize, mut na: usize, ssb: usize, mut nb: usize) -> Result<(), E> {
        let tmp: Vec<usize> = self.a[ssa..ssa + na].to_vec();
        let mut dest = ssa;
        let mut pa = 0;
        let mut pb = ssb;
        let exit = 'merge: {
            self.a[dest] = self.a[pb];
            dest += 1;
            pb += 1;
            nb -= 1;
            if nb == 0 {
                break 'merge Exit::Succeed;
            }
            if na == 1 {
                break 'merge Exit::CopyRest;
            }
            let mut min_gallop = self.min_gallop;
            loop {
                let mut acount = 0;
                let mut bcount = 0;
                // One element at a time until one run keeps winning.
                loop {
                    if (self.less)(self.a[pb], tmp[pa])? {
                        self.a[dest] = self.a[pb];
                        dest += 1;
                        pb += 1;
                        bcount += 1;
                        acount = 0;
                        nb -= 1;
                        if nb == 0 {
                            break 'merge Exit::Succeed;
                        }
                        if bcount >= min_gallop {
                            break;
                        }
                    } else {
                        self.a[dest] = tmp[pa];
                        dest += 1;
                        pa += 1;
                        acount += 1;
                        bcount = 0;
                        na -= 1;
                        if na == 1 {
                            break 'merge Exit::CopyRest;
                        }
                        if acount >= min_gallop {
                            break;
                        }
                    }
                }
                // Gallop until neither run wins by MIN_GALLOP at a time.
                min_gallop += 1;
                loop {
                    min_gallop -= usize::from(min_gallop > 1);
                    self.min_gallop = min_gallop;
                    let k = gallop_right(&mut self.less, self.a[pb], &tmp[pa..pa + na], 0)?;
                    acount = k;
                    if k > 0 {
                        self.a[dest..dest + k].copy_from_slice(&tmp[pa..pa + k]);
                        dest += k;
                        pa += k;
                        na -= k;
                        if na == 1 {
                            break 'merge Exit::CopyRest;
                        }
                        // Impossible with a consistent comparison, which this is not.
                        if na == 0 {
                            break 'merge Exit::Succeed;
                        }
                    }
                    self.a[dest] = self.a[pb];
                    dest += 1;
                    pb += 1;
                    nb -= 1;
                    if nb == 0 {
                        break 'merge Exit::Succeed;
                    }
                    let k = gallop_left(&mut self.less, tmp[pa], &self.a[pb..pb + nb], 0)?;
                    bcount = k;
                    if k > 0 {
                        self.a.copy_within(pb..pb + k, dest);
                        dest += k;
                        pb += k;
                        nb -= k;
                        if nb == 0 {
                            break 'merge Exit::Succeed;
                        }
                    }
                    self.a[dest] = tmp[pa];
                    dest += 1;
                    pa += 1;
                    na -= 1;
                    if na == 1 {
                        break 'merge Exit::CopyRest;
                    }
                    if acount < MIN_GALLOP && bcount < MIN_GALLOP {
                        break;
                    }
                }
                // Penalize leaving galloping mode.
                min_gallop += 1;
                self.min_gallop = min_gallop;
            }
        };
        match exit {
            Exit::Succeed => self.a[dest..dest + na].copy_from_slice(&tmp[pa..pa + na]),
            Exit::CopyRest => {
                // The last element of the copied run goes after everything left of b.
                self.a.copy_within(pb..pb + nb, dest);
                self.a[dest + nb] = tmp[pa];
            }
        }
        Ok(())
    }

    /// As `merge_lo`, right to left, with the second run copied aside. `na > nb`.
    fn merge_hi(&mut self, ssa: usize, na: usize, ssb: usize, nb: usize) -> Result<(), E> {
        let tmp: Vec<usize> = self.a[ssb..ssb + nb].to_vec();
        // Signed, because each cursor steps one below its run's first element at the end.
        let at = |i: isize| i as usize;
        let (mut na, mut nb) = (na as isize, nb as isize);
        let mut dest = (ssb as isize) + nb - 1;
        let mut pb = nb - 1;
        let mut pa = (ssa as isize) + na - 1;
        let exit = 'merge: {
            self.a[at(dest)] = self.a[at(pa)];
            dest -= 1;
            pa -= 1;
            na -= 1;
            if na == 0 {
                break 'merge Exit::Succeed;
            }
            if nb == 1 {
                break 'merge Exit::CopyRest;
            }
            let mut min_gallop = self.min_gallop;
            loop {
                let mut acount: isize = 0;
                let mut bcount: isize = 0;
                loop {
                    if (self.less)(tmp[at(pb)], self.a[at(pa)])? {
                        self.a[at(dest)] = self.a[at(pa)];
                        dest -= 1;
                        pa -= 1;
                        acount += 1;
                        bcount = 0;
                        na -= 1;
                        if na == 0 {
                            break 'merge Exit::Succeed;
                        }
                        if acount >= min_gallop as isize {
                            break;
                        }
                    } else {
                        self.a[at(dest)] = tmp[at(pb)];
                        dest -= 1;
                        pb -= 1;
                        bcount += 1;
                        acount = 0;
                        nb -= 1;
                        if nb == 1 {
                            break 'merge Exit::CopyRest;
                        }
                        if bcount >= min_gallop as isize {
                            break;
                        }
                    }
                }
                min_gallop += 1;
                loop {
                    min_gallop -= usize::from(min_gallop > 1);
                    self.min_gallop = min_gallop;
                    let k = gallop_right(
                        &mut self.less,
                        tmp[at(pb)],
                        &self.a[ssa..ssa + at(na)],
                        at(na - 1),
                    )?;
                    let k = na - k as isize;
                    acount = k;
                    if k > 0 {
                        dest -= k;
                        pa -= k;
                        self.a.copy_within(at(pa + 1)..at(pa + 1 + k), at(dest + 1));
                        na -= k;
                        if na == 0 {
                            break 'merge Exit::Succeed;
                        }
                    }
                    self.a[at(dest)] = tmp[at(pb)];
                    dest -= 1;
                    pb -= 1;
                    nb -= 1;
                    if nb == 1 {
                        break 'merge Exit::CopyRest;
                    }
                    let k =
                        gallop_left(&mut self.less, self.a[at(pa)], &tmp[..at(nb)], at(nb - 1))?;
                    let k = nb - k as isize;
                    bcount = k;
                    if k > 0 {
                        dest -= k;
                        pb -= k;
                        self.a[at(dest + 1)..at(dest + 1 + k)]
                            .copy_from_slice(&tmp[at(pb + 1)..at(pb + 1 + k)]);
                        nb -= k;
                        if nb == 1 {
                            break 'merge Exit::CopyRest;
                        }
                        // Impossible with a consistent comparison, which this is not.
                        if nb == 0 {
                            break 'merge Exit::Succeed;
                        }
                    }
                    self.a[at(dest)] = self.a[at(pa)];
                    dest -= 1;
                    pa -= 1;
                    na -= 1;
                    if na == 0 {
                        break 'merge Exit::Succeed;
                    }
                    if acount < MIN_GALLOP as isize && bcount < MIN_GALLOP as isize {
                        break;
                    }
                }
                min_gallop += 1;
                self.min_gallop = min_gallop;
            }
        };
        match exit {
            Exit::Succeed => {
                if nb > 0 {
                    self.a[at(dest - (nb - 1))..=at(dest)].copy_from_slice(&tmp[..at(nb)]);
                }
            }
            Exit::CopyRest => {
                // The first element of the copied run goes before everything left of a.
                self.a
                    .copy_within(at(pa + 1 - na)..at(pa + 1), at(dest + 1 - na));
                self.a[at(dest - na)] = tmp[at(pb)];
            }
        }
        Ok(())
    }
}

/// The "power" of the boundary between a run at `s1` of length `n1` and the run of length
/// `n2` after it, in a list of `n`: the first bit at which the two runs' midpoints, as
/// fractions of the list, differ.
fn powerloop(s1: usize, n1: usize, n2: usize, n: usize) -> u32 {
    let mut result = 0;
    let mut a = 2 * s1 + n1;
    let mut b = a + n1 + n2;
    loop {
        result += 1;
        if a >= n {
            a -= n;
            b -= n;
        } else if b >= n {
            break;
        }
        a <<= 1;
        b <<= 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn by_float(keys: &[f64]) -> Vec<usize> {
        sorted(keys.len(), |i, j| Ok::<bool, ()>(keys[i] < keys[j])).unwrap()
    }

    #[test]
    fn a_total_order_sorts_stably() {
        let keys: Vec<f64> = (0..500).map(|n| f64::from((n * 7919) % 53)).collect();
        let mut want: Vec<usize> = (0..keys.len()).collect();
        want.sort_by(|i, j| keys[*i].total_cmp(&keys[*j]));
        assert_eq!(by_float(&keys), want);
        assert_eq!(by_float(&[]), Vec::<usize>::new());
        assert_eq!(by_float(&[1.0]), vec![0]);
    }

    /// `sorted(range(len(xs)), key=xs.__getitem__)` under Python 3.14, where a stable sort by
    /// a total order would move the NaN or the elements around it.
    #[test]
    fn nan_keys_come_out_where_python_leaves_them() {
        let nan = f64::NAN;
        assert_eq!(by_float(&[nan, -0.5]), vec![0, 1]);
        assert_eq!(by_float(&[3.0, nan, 1.0]), vec![0, 1, 2]);
        assert_eq!(by_float(&[3.0, 2.0, nan, 1.0]), vec![1, 2, 3, 0]);
        assert_eq!(by_float(&[2.0, nan, 1.0, 0.0]), vec![3, 0, 1, 2]);
    }

    #[test]
    fn an_error_from_the_comparison_stops_the_sort() {
        let got = sorted(3, |i, j| if i + j == 3 { Err("boom") } else { Ok(i < j) });
        assert_eq!(got, Err("boom"));
    }
}
