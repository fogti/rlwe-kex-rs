use core::ops::{AddAssign, SubAssign, MulAssign, Mul, BitXor};
use core::fmt;
use rand::Rng;

// polynomials modulo (2 ^^ n + 1)
// maybe use modulo 251 (largest prime smaller than 256)
// but for debugging we resort to 5

const Q: u8 = 251;

#[derive(Clone, Copy)]
struct Poly<T>([T; 128]);

impl Default for Poly<u8> {
    fn default() -> Self {
        Self([0; 128])
    }
}

impl Default for Poly<bool> {
    fn default() -> Self {
        Self([false; 128])
    }
}

fn modulo_add64(a: u32, b: u32) -> u8 {
    ((a + b) % (Q as u32)) as u8
}

fn modulo_add(a: u8, b: u8) -> u8 {
    modulo_add64(a as u32, b as u32)
}

impl AddAssign for Poly<u8> {
    fn add_assign(&mut self, other: Self) {
        for (i, j) in self.0.iter_mut().zip(other.0.iter()) {
            *i = modulo_add(*i, *j);
        }
    }
}

impl SubAssign for Poly<u8> {
    fn sub_assign(&mut self, other: Self) {
        for (i, j) in self.0.iter_mut().zip(other.0.iter()) {
            *i = ((Q as i32 + *i as i32 - *j as i32) as u32 % (Q as u32)) as u8;
        }
    }
}

impl Mul for &Poly<u8> {
    type Output = Poly<u8>;
    fn mul(self, other: Self) -> Poly<u8> {
        // calculate product
        let mut tmp = [0u8; 255];
        for (n, i) in self.0.into_iter().enumerate() {
            let i_ = i as u32;
            for (m, j) in other.0.into_iter().enumerate() {
                let tnm = &mut tmp[n + m];
                *tnm = modulo_add64(*tnm as u32, i_ * (j as u32));
            }
        }

        // calculate remainder against (x^(64) + 1)
        let mut ret = Poly::default();
        let rl = ret.0.len();
        let (a, b) = tmp.split_at(rl - 1);
        ret.0.copy_from_slice(b);
        for (n, &i) in a.iter().enumerate() {
            // n=0 @ 126 -> m=1 @ 62
            let rdi = &mut ret.0[1 + n];
            *rdi = modulo_add(Q - i, *rdi);
        }
        ret
    }
}

impl MulAssign<u8> for Poly<u8> {
    fn mul_assign(&mut self, other: u8) {
        let o_ = other as u32;
        self.0.iter_mut().for_each(|i| *i = modulo_add64(0, (*i as u32) * o_));
    }
}

impl BitXor for &Poly<bool> {
    type Output = Poly<bool>;
    fn bitxor(self, rhs: Self) -> Poly<bool> {
        let mut ret = Poly::default();
        ret.0.iter_mut().zip(self.0.iter().zip(rhs.0.iter()))
            .for_each(|(r, (&i, &j))| *r = i != j);
        ret
    }
}

impl fmt::Display for Poly<bool> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &i in self.0.iter() {
            f.write_str(if i { "*" } else { " " })?;
        }
        Ok(())
    }
}

impl fmt::Display for Poly<u8> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in self.0.iter() {
            write!(f, "{:02x}", i)?;
        }
        Ok(())
    }
}

impl Poly<u8> {
    fn random<R: Rng>(rng: &mut R) -> Self {
        let mut ret = Poly::default();
        ret.0.iter_mut().for_each(|i| *i = rng.gen_range(0..Q));
        ret
    }
}

#[derive(Clone)]
struct Sep {
    s: Poly<u8>,
    e: Poly<u8>,
    p: Poly<u8>,
}

impl fmt::Display for Sep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "         s: {}", self.s)?;
        writeln!(f, "         e: {}", self.e)?;
        writeln!(f, "         p: {}", self.p)?;
        Ok(())
    }
}

// primary functions

fn gen_noise<R: Rng>(rng: &mut R) -> Poly<u8> {
    let mut ret = Poly::default();
    ret.0.iter_mut().for_each(|i| *i = rng.gen_range(0..Q / 16));
    ret
}

fn gen_sep<R: Rng>(a: &Poly<u8>, rng: &mut R) -> Sep {
    let s = Poly::random(rng);
    let e = gen_noise(rng);
    let mut e2 = e;
    e2 *= 2;
    let mut p = a * &s;
    p += e2;
    Sep { s, e, p }
}

fn compute_w(Poly(p): Poly<u8>) -> Poly<bool> {
    let q4 = Q / 4;
    let _3q4 = (3 * (Q as u32) / 4) as u8;
    let mut w = Poly::default();
    let sig = |i: u8| !(q4 <= i && i < _3q4);
    for (i, j) in w.0.iter_mut().zip(p.iter()) {
        assert!(*j < Q);
        *i = sig(*j);
    }
    w
}

fn compute_sks(w: &Poly<bool>, s: &Poly<u8>, p: &Poly<u8>) -> Poly<bool> {
    let q18 = Q / 8;
    let q38 = (3 * (Q as u32) / 8) as u8;
    let q58 = (5 * (Q as u32) / 8) as u8;
    let q78 = (7 * (Q as u32) / 8) as u8;

    let mut ret = Poly::default();
    let tmp = s * p;

    for (r, (i, &j)) in ret.0.iter_mut().zip(w.0.iter().zip(tmp.0.iter())) {
        *r = if !*i {
            // region (q/4..3q/4)
            !(q38 <= j && j < q58)
        } else {
            // region (q/4..q/2), (3q/4..q)
            !(q18 <= j && j < q78)
        }
    }
    ret
}

fn main() {
    let mut rng = rand::thread_rng();

    // generate A
    let shr_a = Poly::random(&mut rng);
    println!("A         = {}", shr_a);

    // Alice
    let a_sep = gen_sep(&shr_a, &mut rng);
    println!("Alice:");
    print!("{}", a_sep);

    // Bob
    let b_sep = gen_sep(&shr_a, &mut rng);
    println!("Bob:");
    println!("{}", b_sep);

    // compute w for error rounding
    let w = compute_w(b_sep.p);
    println!("w         = {}", w);
    let w2 = compute_w(a_sep.p);
    println!("w'        = {}", w2);
    println!("w delta   = {}", &w ^ &w2);

    // shared secret + rounding
    let a_sks = compute_sks(&w, &a_sep.s, &b_sep.p);
    println!("sks Alice = {}", a_sks);
    let b_sks = compute_sks(&w, &b_sep.s, &a_sep.p);
    println!("sks Bob   = {}", b_sks);
    let dif = &a_sks ^ &b_sks;
    println!("delta     = {}", dif);

    let mut ed = a_sep.e;
    ed += b_sep.e;
    ed *= 2;
    println!("A-B E   d = {}", ed);

    if a_sks.0 != b_sks.0 {
        println!("sks mismatch");

        let q18 = Q / 8;
        let q38 = (3 * (Q as u32) / 8) as u8;
        let q58 = (5 * (Q as u32) / 8) as u8;
        let q78 = (7 * (Q as u32) / 8) as u8;
        println!("q1/8 = {}; q3/8 = {}; q5/8 = {}; q7/8 = {}", q18, q38, q58, q78);
    }
}
