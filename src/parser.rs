use crate::statement::{Exec, Join, Statement as Stt};
use gobble::*;

parser! {(SJoin ->Join)
    or!(
        "|".asv(Join::Pipe),
        "^|".asv(Join::PipeErr),
    )
}

parser! {(Statement->Stt)
    ExecRight.map(|a|Stt::Exec(a))
}

parser! {(ExecRight->Exec)
    (ExecLeft,maybe((ws__(SJoin),ExecRight))).map(|(a,op_b)|match op_b{
        Some((jn,b))=>Exec::Join(jn,Box::new(a),Box::new(b)),
        None=>a,
    })
}

parser! {(ExecLeft->Exec)
    (common::Ident , star(ws_(ArgP))).map(|(c,a)|Exec::Simple(c,a))
}

parser! { (ArgP->String)
    or!( (Alpha,NumDigit,"_.-/").plus(),common::Quoted)
}
