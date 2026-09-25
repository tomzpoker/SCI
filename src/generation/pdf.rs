use chrono::NaiveDate;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

pub struct PdfSpec {
    pub header: String,
    pub footer: String,
    pub reference: String,
    pub status: String,
    pub date: NaiveDate,
    pub lines: Vec<String>,
    pub watermark: String,
    pub logo_path: Option<std::path::PathBuf>,
    pub margins_mm: (f64, f64, f64, f64),
}

fn winansi_bytes(s: &str) -> Vec<u8> {
    s.chars().map(|ch| match ch {
        'é'=>0xE9,'è'=>0xE8,'ê'=>0xEA,'ë'=>0xEB,'à'=>0xE0,'â'=>0xE2,'ä'=>0xE4,
        'î'=>0xEE,'ï'=>0xEF,'ô'=>0xF4,'ö'=>0xF6,'ù'=>0xF9,'û'=>0xFB,'ü'=>0xFC,
        'ç'=>0xE7,'É'=>0xC9,'È'=>0xC8,'Ê'=>0xCA,'À'=>0xC0,'Â'=>0xC2,'Î'=>0xCE,
        'Ô'=>0xD4,'Ù'=>0xD9,'Œ'=>0x8C,'œ'=>0x9C,'€'=>0x80,'’'=>0x92,'–'=>0x96,
        '—'=>0x97,'…'=>0x85, _ if ch.is_ascii()=>ch as u8, _=>b'?'
    }).collect()
}
fn pdf_escape(s: &str) -> String {
    let mut out = String::from("<");
    for b in winansi_bytes(s) { out.push_str(&format!("{:02X}", b)); }
    out.push('>');
    out
}

fn wrap_line(s:&str,max:usize)->Vec<String>{
    if s.chars().count()<=max { return vec![s.to_string()]; }
    let mut out=Vec::new(); let mut cur=String::new();
    for word in s.split_whitespace(){
        if !cur.is_empty() && cur.chars().count()+1+word.chars().count()>max { out.push(cur); cur=String::new(); }
        if !cur.is_empty(){cur.push(' ');} cur.push_str(word);
    }
    if !cur.is_empty(){out.push(cur);} if out.is_empty(){out.push(s.chars().take(max).collect());} out
}
fn jpeg_info(bytes:&[u8])->Option<(usize,usize,usize)>{
    if bytes.len()<4||bytes[0]!=0xFF||bytes[1]!=0xD8{return None;} let mut i=2;
    while i+9<bytes.len(){
        if bytes[i]!=0xFF{i+=1;continue;} while i<bytes.len()&&bytes[i]==0xFF{i+=1;} if i>=bytes.len(){break;}
        let marker=bytes[i]; i+=1; if marker==0xD9||marker==0xDA{break;} if i+2>bytes.len(){break;}
        let len=u16::from_be_bytes([bytes[i],bytes[i+1]]) as usize; if len<2||i+len>bytes.len(){break;}
        if matches!(marker,0xC0|0xC1|0xC2|0xC3|0xC5|0xC6|0xC7|0xC9|0xCA|0xCB|0xCD|0xCE|0xCF){
            return Some((u16::from_be_bytes([bytes[i+5],bytes[i+6]]) as usize,u16::from_be_bytes([bytes[i+3],bytes[i+4]]) as usize,bytes[i+7] as usize));
        }
        i+=len;
    }
    None
}

pub fn write_pdf(path:&Path,spec:&PdfSpec)->io::Result<()> {
    let (_top,_right,_bottom,_left)=spec.margins_mm;
    let mut wrapped_lines=Vec::new(); for l in &spec.lines { wrapped_lines.extend(wrap_line(l,92)); }
    if wrapped_lines.is_empty(){wrapped_lines.push(String::new());}
    let mut pages:Vec<Vec<String>>=Vec::new();
    for chunk in wrapped_lines.chunks(43){ pages.push(chunk.to_vec()); }
    let n=pages.len();
    let mut jpeg=Vec::new(); let mut logo_dims=None;
    if let Some(p)=&spec.logo_path { if p.extension().and_then(|e|e.to_str()).map(|e|e.eq_ignore_ascii_case("jpg")||e.eq_ignore_ascii_case("jpeg")).unwrap_or(false) { if let Ok(data)=fs::read(p) { if let Some(d)=jpeg_info(&data) { logo_dims=Some(d); jpeg=data; } } } }
    let font_obj=3usize;
    let image_obj=if logo_dims.is_some(){Some(4usize)}else{None};
    let first_page_obj=if image_obj.is_some(){5usize}else{4usize};
    let mut kids=String::new(); for i in 0..n { if i>0 {kids.push(' ');} kids.push_str(&format!("{} 0 R",first_page_obj+i*2)); }
    let mut objects:Vec<Vec<u8>>=Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(format!("<< /Type /Pages /Kids [{}] /Count {} >>",kids,n).into_bytes());
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec());
    if let Some((w,h,c))=logo_dims { let cs=if c==1{"/DeviceGray"}else{"/DeviceRGB"}; let mut o=format!("<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace {} /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",w,h,cs,jpeg.len()).into_bytes(); o.extend_from_slice(&jpeg); o.extend_from_slice(b"\nendstream"); objects.push(o); }
    for (idx,page_lines) in pages.iter().enumerate(){
        let page_obj=first_page_obj+idx*2; let content_obj=page_obj+1;
        let mut content=String::new();
        content.push_str("BT /F1 10 Tf 50 810 Td "); content.push_str(&pdf_escape(&spec.header)); content.push_str(" Tj ET\n");
        content.push_str("BT /F1 9 Tf 50 790 Td "); content.push_str(&pdf_escape(&format!("Référence : {}",spec.reference))); content.push_str(" Tj ET\n");
        content.push_str("BT /F1 9 Tf 440 790 Td "); content.push_str(&pdf_escape(&spec.date.to_string())); content.push_str(" Tj ET\n");
        if let Some((w,h,_))=logo_dims { let scale_w=85.0; let scale_h=scale_w*(h as f64)/(w as f64).max(1.0); let y=760.0-scale_h; content.push_str(&format!("q {} 0 0 {} 485 {} cm /Im1 Do Q\n",scale_w,scale_h,y)); }
        for (li,line) in page_lines.iter().enumerate(){let y=760-(li as i32)*15;content.push_str(&format!("BT /F1 9 Tf 50 {} Td {} Tj ET\n",y,pdf_escape(line)));}
        if !spec.watermark.is_empty(){content.push_str("BT /F1 28 Tf 0.88 g 135 420 Td ");content.push_str(&pdf_escape(&spec.watermark));content.push_str(" Tj ET\n0 g\n");}
        let footer=spec.footer.replace("{reference}",&spec.reference).replace("{status}",&spec.status).replace("{page}",&(idx+1).to_string()).replace("{pages}",&n.to_string());
        content.push_str("BT /F1 8 Tf 50 24 Td ");content.push_str(&pdf_escape(&footer));content.push_str(" Tj ET\n");
        let resources=if image_obj.is_some(){"<< /Font << /F1 3 0 R >> /XObject << /Im1 4 0 R >> >>"}else{"<< /Font << /F1 3 0 R >> >>"};
        let page_dict=format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources {} /Contents {} 0 R >>",resources,content_obj);
        objects.push(page_dict.into_bytes());
        let mut co=format!("<< /Length {} >>\nstream\n",content.as_bytes().len()).into_bytes(); co.extend_from_slice(content.as_bytes()); co.extend_from_slice(b"\nendstream"); objects.push(co);
    }
    let mut pdf=b"%PDF-1.4\n".to_vec(); let mut offsets=Vec::with_capacity(objects.len()+1); offsets.push(0);
    for (idx,obj) in objects.iter().enumerate(){offsets.push(pdf.len());pdf.extend_from_slice(format!("{} 0 obj\n",idx+1).as_bytes());pdf.extend_from_slice(obj);pdf.extend_from_slice(b"\nendobj\n");}
    let xref=pdf.len(); pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n",objects.len()+1).as_bytes()); for off in offsets.iter().skip(1){pdf.extend_from_slice(format!("{:010} 00000 n \n",off).as_bytes());}
    pdf.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",objects.len()+1,xref).as_bytes());
    let mut f=fs::File::create(path)?;f.write_all(&pdf)?;Ok(())
}

#[cfg(test)]
mod tests { use super::*; #[test] fn escapes_pdf(){assert_eq!(pdf_escape("Été"),"<C974E9>");} #[test] fn wraps(){assert!(wrap_line("un deux trois quatre cinq six",8).len()>1);} }
