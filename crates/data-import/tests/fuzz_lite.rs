//! Fuzz-lite deterministik untuk impor CSV — tanpa dependency, tanpa jaringan.
//!
//! Bukan pengganti `cargo-fuzz`; ini jaring pengaman cepat yang selalu ikut
//! `cargo test`: invariant yang dijaga adalah **tidak pernah panic/hang** pada
//! input arbitrer, dan setiap hasil `Ok` berisi bar valid yang naik monoton.

use sokanlab_data_import::{parse_csv_ohlcv, preview_csv};

/// Xorshift64* — deterministik agar kegagalan bisa direproduksi dari seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn random_char(rng: &mut Rng) -> char {
    const POOL: &[char] = &[
        '0', '1', '9', '-', '+', '.', ',', 'e', 'E', '\n', '\r', ' ', '"', '\'', '\t', '{', '}',
        'x', 'é', '中', '😀',
    ];
    POOL[rng.below(POOL.len())]
}

const CORPUS: &[&str] = &[
    // Valid
    "timestamp,open,high,low,close,volume\n1700000000,100,110,90,105,1000\n1700000060,105,115,95,110,1200\n",
    // Kosong / header saja / tanpa header
    "",
    "\n",
    "timestamp,open,high,low,close,volume\n",
    "1700000000,100,110,90,105,1000\n",
    // Header salah
    "a,b,c\n1,2,3\n",
    // Jumlah kolom ganjil
    "timestamp,open,high,low,close,volume\n1700000000,100,110,90,105\n",
    // Nilai kosong / non-angka / spasi
    "timestamp,open,high,low,close,volume\n1700000000,,110,90,105,1\n",
    "timestamp,open,high,low,close,volume\n1700000000,abc,110,90,105,1\n",
    "timestamp,open,high,low,close,volume\n 1700000000 , 100 , 110 , 90 , 105 , 1 \n",
    // Duplikat & non-monoton
    "timestamp,open,high,low,close,volume\n1700000000,100,110,90,105,1\n1700000000,100,110,90,105,1\n",
    "timestamp,open,high,low,close,volume\n1700000060,100,110,90,105,1\n1700000000,100,110,90,105,1\n",
    // OHLC tidak konsisten (high < low, open di luar rentang, volume negatif)
    "timestamp,open,high,low,close,volume\n1700000000,100,90,110,105,1\n",
    "timestamp,open,high,low,close,volume\n1700000000,100,110,90,105,-1\n",
    // Nilai ekstrem
    "timestamp,open,high,low,close,volume\n9223372036854775807,100,110,90,105,1\n",
    "timestamp,open,high,low,close,volume\n-9223372036854775808,100,110,90,105,1\n",
    "timestamp,open,high,low,close,volume\n1700000000000000000,100,110,90,105,1\n",
    // Koma berlebih / baris kosong di tengah / CRLF
    "timestamp,open,high,low,close,volume\r\n1700000000,100,110,90,105,1\r\n\r\n1700000060,105,115,95,110,1\r\n",
];

fn mutate(rng: &mut Rng, base: &str) -> String {
    let mut chars: Vec<char> = base.chars().collect();
    let ops = 1 + rng.below(8);
    for _ in 0..ops {
        match rng.below(6) {
            0 => {
                if !chars.is_empty() {
                    let i = rng.below(chars.len());
                    chars.remove(i);
                }
            }
            1 => {
                let i = rng.below(chars.len() + 1);
                let c = random_char(rng);
                chars.insert(i, c);
            }
            2 => {
                if !chars.is_empty() {
                    let i = rng.below(chars.len());
                    chars[i] = random_char(rng);
                }
            }
            3 => {
                let keep = rng.below(chars.len() + 1);
                chars.truncate(keep);
            }
            4 => {
                let s: String = chars.iter().collect();
                let li = rng.below(s.split('\n').count());
                let dup = s.split('\n').nth(li).unwrap_or("").to_string();
                let mut out = s;
                out.push('\n');
                out.push_str(&dup);
                chars = out.chars().collect();
            }
            _ => {
                let n = rng.below(10);
                for _ in 0..n {
                    let c = random_char(rng);
                    chars.push(c);
                }
            }
        }
    }
    chars.into_iter().collect()
}

/// Invariant utama: tak pernah panic; `Ok` ⇒ bar valid & ts naik ketat.
fn check(text: &str, seed: u64, desc: &str) {
    if let Ok(bars) = parse_csv_ohlcv(text) {
        assert!(!bars.is_empty(), "Ok tapi kosong (seed {seed}, {desc})");
        for (i, b) in bars.iter().enumerate() {
            assert!(
                b.validate().is_ok(),
                "bar tak valid index {i} (seed {seed}, {desc}): {b:?}"
            );
        }
        for w in bars.windows(2) {
            assert!(
                w[0].ts_millis < w[1].ts_millis,
                "ts tidak naik ketat (seed {seed}, {desc})"
            );
        }
    }
    // Preview juga tidak boleh panic pada input apa pun.
    let _ = preview_csv(text, 3);
}

#[test]
fn mutasi_corpus_tidak_pernah_panic() {
    let mut rng = Rng(0x5A4B_4C41_4230_3031); // "SOKLAB001"
    for i in 0..1500 {
        let base = CORPUS[i % CORPUS.len()];
        let text = mutate(&mut rng, base);
        check(&text, rng.0, "mutasi corpus");
    }
}

#[test]
fn string_acak_tidak_pernah_panic() {
    let mut rng = Rng(0xDEAD_BEEF_CAFE_F00D);
    for _ in 0..800 {
        let n = rng.below(300);
        let mut s = String::new();
        for _ in 0..n {
            let c = random_char(&mut rng);
            s.push(c);
        }
        check(&s, rng.0, "string acak");
    }
}

#[test]
fn timestamp_ekstrem_i64_tidak_panic() {
    // Regresi: i64::MIN dan nilai dekat batas tidak boleh memicu overflow
    // pada konversi detik→milidetik (`abs`/`*1000`).
    for ts in [
        "-9223372036854775808",
        "9223372036854775807",
        "-999999999999",
        "999999999999",
    ] {
        let text = format!("timestamp,open,high,low,close,volume\n{ts},100,110,90,105,1\n");
        let _ = parse_csv_ohlcv(&text);
        let _ = preview_csv(&text, 3);
    }
}
