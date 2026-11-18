type Q = [f64; 4];
fn n2(q: Q) -> f64 { q[0]*q[0]+q[1]*q[1]+q[2]*q[2]+q[3]*q[3] }
fn scale(q: Q, s: f64) -> Q { q.map(|v| v*s) }
fn conj(q: Q) -> Q { [q[0],-q[1],-q[2],-q[3]] }
fn mul(a: Q, b: Q) -> Q {
 [a[0]*b[0]-a[1]*b[1]-a[2]*b[2]-a[3]*b[3],
  a[0]*b[1]+a[1]*b[0]+a[2]*b[3]-a[3]*b[2],
  a[0]*b[2]-a[1]*b[3]+a[2]*b[0]+a[3]*b[1],
  a[0]*b[3]+a[1]*b[2]-a[2]*b[1]+a[3]*b[0]]
}
fn rotate(q: Q, v: [f64;3]) -> [f64;3] { let r=mul(mul(q,[0.0,v[0],v[1],v[2]]),conj(q)); [r[1],r[2],r[3]] }
fn main() {
 let id=[1.0,0.0,0.0,0.0]; let a=[1.0,-0.0,0.0,0.0];
 println!("signed-zero right identity: {:?} -> {:?}", a.map(f64::to_bits),mul(a,id).map(f64::to_bits));
 let mut state=123456789_u64;
 let mut random=|| {state ^= state << 13; state ^= state >> 7; state ^= state << 17; (state>>11) as f64/9007199254740992.0*2.0-1.0};
 let mut accepted=0;let mut escaped=0;let mut first=None;
 for _ in 0..1_000_000 {
  let a=[random(),random(),random(),random()];
  let q=scale(scale(a,1.0/libm::sqrt(n2(a))),1.0+random()*8.0*f64::EPSILON);
  if (n2(q)-1.0).abs()>8.0*f64::EPSILON {continue;}
  accepted+=1;
  let inv=scale(conj(q),1.0/n2(q));
  if (n2(inv)-1.0).abs()>8.0*f64::EPSILON {escaped+=1;if first.is_none(){first=Some((q,(n2(q)-1.0)/f64::EPSILON,(n2(inv)-1.0)/f64::EPSILON));}}
 }
 println!("literal conjugate/n2 band escapes={escaped}/{accepted}; first={first:?}");
 let q=[1.0+4.0*f64::EPSILON,0.0,0.0,0.0];let v=[1.0,0.0,0.0];
 let q_inverse=scale(conj(q),1.0/n2(q));
 println!("q={q:?} n2={:.17}; literal q/n2 action={:?}; report translation/n2 action={:?}",n2(q),rotate(q_inverse,v),rotate(conj(q),v).map(|x|x/n2(q)));
 for old in [1_u64,2,100,1_700_000_000_000_000_000] {
  let bytes=postcard::to_allocvec(&old).unwrap();let decoded: i64=postcard::from_bytes(&bytes).unwrap();
  println!("postcard old u64={old} decoded as i64={decoded}");
 }
 let nanos=i64::MIN;let secs=nanos.div_euclid(1_000_000_000);let frac=nanos.rem_euclid(1_000_000_000) as u32;
 let narrow=secs.checked_mul(1_000_000_000).and_then(|x| x.checked_add(i64::from(frac)));
 let wide=i64::try_from(i128::from(secs)*1_000_000_000+i128::from(frac));
 println!("MIN seconds split=({secs},{frac}) narrow={narrow:?} wide={wide:?}; MAX-MIN={}",i64::MAX.abs_diff(i64::MIN));
 let dt=|a:u64,b:u64| (a-b)*1_000_000_000/32768;
 let stamp=|ticks:u64| ticks*1_000_000_000/32768;
 let direct=dt(2,1) as f64/dt(3,1) as f64;
 let absolute=(stamp(2)-stamp(1)) as f64/(stamp(3)-stamp(1)) as f64;
 println!("32768Hz floor Duration-per-difference ratio={direct:.17} floor-stamp ratio={absolute:.17}");
}
