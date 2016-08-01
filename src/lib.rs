//! A crate which provides low-level Rust API of Gurobi Optimizer.
//!
//! This crate provides wrappers of the Gurobi solver which supports some
//! types of mathematical programming problems (e.g. Linear programming; LP,
//! Mixed Integer Linear Programming; MILP, and so on).
//!
//! ## Installation
//! Before using this crate, you should install Gurobi and obtain a license.
//! The instruction can be found
//! [here](http://www.gurobi.com/downloads/licenses/license-center).
//!
//! ## Examples
//! Work in progress...

extern crate gurobi_sys as ffi;

mod error;
mod util;

// re-exports
pub use error::{Error, Result};
pub use VarType::*;
pub use ConstrSense::*;
pub use ModelSense::*;
pub use ffi::{IntParam, DoubleParam, StringParam};
pub use ffi::{IntAttr, DoubleAttr, CharAttr, StringAttr};

// internal imports
use std::ptr::{null, null_mut};
use util::*;
use std::ffi::CString;

#[derive(Debug)]
pub enum VarType {
  Binary,
  Continuous(f64, f64),
  Integer(i64, i64),
}

pub enum ConstrSense {
  Equal,
  Greater,
  Less,
}

pub enum ModelSense {
  Minimize,
  Maximize,
}


/// Gurobi environment object
pub struct Env {
  env: *mut ffi::GRBenv,
}


/// A trait which implemements helper functions for C API.
pub trait HasGetEnv {
  unsafe fn get_env(&self) -> *mut ffi::GRBenv;
  fn get_error_msg(&self) -> String;
}

impl HasGetEnv for Env {
  unsafe fn get_env(&self) -> *mut ffi::GRBenv {
    self.env
  }

  fn get_error_msg(&self) -> String {
    get_error_msg_env(self.env)
  }
}

/// provides function to query/set the value of parameters.
pub trait HasParam<P>: HasGetEnv
  where CString: From<P>
{
  type Output;

  /// Query the value of a parameter.
  fn get(&self, param: P) -> Result<Self::Output> {
    let mut value = Self::init();
    let error = unsafe {
      Self::get_param(self.get_env(),
                      CString::from(param).as_ptr(),
                      Self::as_rawfrom(&mut value))
    };
    if error != 0 {
      return Err(Error::FromAPI(self.get_error_msg(), error));
    }
    Ok(Self::to_out(value))
  }

  /// Set the value of a parameter.
  fn set(&mut self, param: P, value: Self::Output) -> Result<()> {
    let error = unsafe {
      Self::set_param(self.get_env(),
                      CString::from(param).as_ptr(),
                      Self::as_rawto(value))
    };
    if error != 0 {
      return Err(Error::FromAPI(self.get_error_msg(), error));
    }
    Ok(())
  }

  // associated types/functions which should be hidden.
  type RawFrom;
  type RawTo;
  type Init;

  fn init() -> Self::Init;
  fn as_rawfrom(val: &mut Self::Init) -> Self::RawFrom;
  fn as_rawto(output: Self::Output) -> Self::RawTo;

  unsafe fn get_param(env: *mut ffi::GRBenv,
                      paramname: ffi::c_str,
                      value: Self::RawFrom)
                      -> ffi::c_int;
  unsafe fn set_param(env: *mut ffi::GRBenv,
                      paramname: ffi::c_str,
                      value: Self::RawTo)
                      -> ffi::c_int;

  fn to_out(init: Self::Init) -> Self::Output;
}


/// provides function to query/set the value of attributes.
pub trait HasAttr<Attr> {
  type Output;

  fn get(&self, attr: Attr) -> Result<Self::Output>;

  fn set(&mut self, attr: Attr, value: Self::Output) -> Result<()>;

  fn get_element(&self, attr: Attr, element: i32) -> Result<Self::Output>;

  fn set_element(&mut self,
                 attr: Attr,
                 element: i32,
                 value: Self::Output)
                 -> Result<()>;

  fn get_array(&self,
               attr: Attr,
               first: usize,
               len: usize)
               -> Result<Vec<Self::Output>>;

  fn set_array(&mut self,
               attr: Attr,
               first: usize,
               values: &[Self::Output])
               -> Result<()>;

  fn get_list(&self, attr: Attr, ind: &[i32]) -> Result<Vec<Self::Output>>;
  fn set_list(&mut self,
              attr: Attr,
              ind: &[i32],
              value: &[Self::Output])
              -> Result<()>;
}

impl Env {
  /// create an environment with log file
  pub fn new(logfilename: &str) -> Result<Env> {
    let mut env = null_mut::<ffi::GRBenv>();
    let logfilename = try!(make_c_str(logfilename));
    let error = unsafe { ffi::GRBloadenv(&mut env, logfilename.as_ptr()) };
    if error != 0 {
      return Err(Error::FromAPI(get_error_msg_env(env), error));
    }
    Ok(Env { env: env })
  }

  /// create an empty model object associted with the environment.
  pub fn new_model(&self, modelname: &str) -> Result<Model> {
    let modelname = try!(make_c_str(modelname));
    let mut model = null_mut::<ffi::GRBmodel>();
    let error = unsafe {
      ffi::GRBnewmodel(self.env,
                       &mut model,
                       modelname.as_ptr(),
                       0,
                       null(),
                       null(),
                       null(),
                       null(),
                       null())
    };
    if error != 0 {
      return Err(Error::FromAPI(self.get_error_msg(), error));
    }

    Ok(Model {
      model: model,
      env: self,
      vars: Vec::new(),
      constrs: Vec::new(),
      qconstrs: Vec::new(),
    })
  }
}

impl Drop for Env {
  fn drop(&mut self) {
    unsafe { ffi::GRBfreeenv(self.env) };
    self.env = null_mut();
  }
}

impl HasParam<IntParam> for Env {
  type Output = i32;

  type Init = i32;
  type RawFrom = *mut ffi::c_int;
  type RawTo = ffi::c_int;

  #[inline(always)]
  unsafe fn get_param(env: *mut ffi::GRBenv,
                      paramname: ffi::c_str,
                      value: *mut ffi::c_int)
                      -> ffi::c_int {
    ffi::GRBgetintparam(env, paramname, value)
  }

  #[inline(always)]
  unsafe fn set_param(env: *mut ffi::GRBenv,
                      paramname: ffi::c_str,
                      value: ffi::c_int)
                      -> ffi::c_int {
    ffi::GRBsetintparam(env, paramname, value)
  }

  #[inline(always)]
  fn init() -> i32 {
    0
  }

  #[inline(always)]
  fn as_rawfrom(val: &mut i32) -> *mut ffi::c_int {
    val
  }

  #[inline(always)]
  fn as_rawto(output: i32) -> ffi::c_int {
    output
  }

  #[inline(always)]
  fn to_out(input: i32) -> i32 {
    input
  }
}

impl HasParam<DoubleParam> for Env {
  type Output = f64;

  type Init = f64;
  type RawFrom = *mut ffi::c_double;
  type RawTo = ffi::c_double;

  #[inline(always)]
  unsafe fn get_param(env: *mut ffi::GRBenv,
                      paramname: ffi::c_str,
                      value: *mut ffi::c_double)
                      -> ffi::c_int {
    ffi::GRBgetdblparam(env, paramname, value)
  }

  #[inline(always)]
  unsafe fn set_param(env: *mut ffi::GRBenv,
                      paramname: ffi::c_str,
                      value: ffi::c_double)
                      -> ffi::c_int {
    ffi::GRBsetdblparam(env, paramname, value)
  }

  #[inline(always)]
  fn init() -> f64 {
    0.0
  }

  #[inline(always)]
  fn as_rawfrom(val: &mut f64) -> *mut ffi::c_double {
    val
  }

  #[inline(always)]
  fn as_rawto(output: f64) -> ffi::c_double {
    output
  }

  #[inline(always)]
  fn to_out(input: f64) -> f64 {
    input
  }
}

impl HasParam<StringParam> for Env {
  type Output = String;

  type Init = Vec<ffi::c_char>;
  type RawFrom = *mut ffi::c_char;
  type RawTo = *const ffi::c_char;

  #[inline(always)]
  unsafe fn get_param(env: *mut ffi::GRBenv,
                      paramname: ffi::c_str,
                      value: *mut ffi::c_char)
                      -> ffi::c_int {
    ffi::GRBgetstrparam(env, paramname, value)
  }

  #[inline(always)]
  unsafe fn set_param(env: *mut ffi::GRBenv,
                      paramname: ffi::c_str,
                      value: *const ffi::c_char)
                      -> ffi::c_int {
    ffi::GRBsetstrparam(env, paramname, value)
  }

  #[inline(always)]
  fn init() -> Vec<ffi::c_char> {
    Vec::with_capacity(4096)
  }

  #[inline(always)]
  fn as_rawfrom(val: &mut Vec<ffi::c_char>) -> *mut ffi::c_char {
    val.as_mut_ptr()
  }

  #[inline(always)]
  fn as_rawto(output: String) -> *const ffi::c_char {
    CString::new(output.as_str()).unwrap().as_ptr()
  }

  #[inline(always)]
  fn to_out(input: Vec<ffi::c_char>) -> String {
    unsafe { util::from_c_str(input.as_ptr()) }
  }
}


/// Gurobi Model
pub struct Model<'a> {
  model: *mut ffi::GRBmodel,
  env: &'a Env,
  vars: Vec<i32>,
  constrs: Vec<i32>,
  qconstrs: Vec<i32>,
}

impl<'a> Model<'a> {
  /// apply all modification of the model to process.
  pub fn update(&mut self) -> Result<()> {
    let error = unsafe { ffi::GRBupdatemodel(self.model) };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(())
  }

  /// create a copy of the model
  pub fn copy(&self) -> Result<Model> {
    let copied = unsafe { ffi::GRBcopymodel(self.model) };
    if copied.is_null() {
      return Err(Error::FromAPI("Failed to create a copy of the model"
                                  .to_owned(),
                                20002));
    }
    Ok(Model {
      env: self.env,
      model: copied,
      vars: self.vars.clone(),
      constrs: self.constrs.clone(),
      qconstrs: self.qconstrs.clone(),
    })
  }

  /// optimize the model.
  pub fn optimize(&mut self) -> Result<()> {
    try!(self.update());

    let error = unsafe { ffi::GRBoptimize(self.model) };
    if error != 0 {
      return Err(self.error_from_api(error));
    }

    Ok(())
  }

  /// write information of the model to file.
  pub fn write(&self, filename: &str) -> Result<()> {
    let filename = try!(make_c_str(filename));
    let error = unsafe { ffi::GRBwrite(self.model, filename.as_ptr()) };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(())
  }

  /// add a decision variable to the model.
  pub fn add_var(&mut self,
                 name: &str,
                 vtype: VarType,
                 obj: f64)
                 -> Result<i32> {
    // extract parameters
    use VarType::*;
    let (vtype, lb, ub) = match vtype {
      Binary => ('B' as ffi::c_char, 0.0, 1.0),
      Continuous(lb, ub) => ('C' as ffi::c_char, lb, ub),
      Integer(lb, ub) => {
        ('I' as ffi::c_char, lb as ffi::c_double, ub as ffi::c_double)
      }
    };
    let name = try!(make_c_str(name));
    let error = unsafe {
      ffi::GRBaddvar(self.model,
                     0,
                     null(),
                     null(),
                     obj,
                     lb,
                     ub,
                     vtype,
                     name.as_ptr())
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }

    let col_no = self.vars.len() as i32;
    self.vars.push(col_no);

    Ok(col_no)
  }

  /// add a linear constraint to the model.
  pub fn add_constr(&mut self,
                    name: &str,
                    ind: &[ffi::c_int],
                    val: &[ffi::c_double],
                    sense: ConstrSense,
                    rhs: ffi::c_double)
                    -> Result<i32> {
    if ind.len() != val.len() {
      return Err(Error::InconsitentDims);
    }

    let sense = match sense {
      ConstrSense::Equal => '=' as ffi::c_char,
      ConstrSense::Less => '<' as ffi::c_char,
      ConstrSense::Greater => '>' as ffi::c_char,
    };
    let constrname = try!(make_c_str(name));

    let error = unsafe {
      ffi::GRBaddconstr(self.model,
                        ind.len() as ffi::c_int,
                        ind.as_ptr(),
                        val.as_ptr(),
                        sense,
                        rhs,
                        constrname.as_ptr())
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }

    let row_no = self.constrs.len() as i32;
    self.constrs.push(row_no);

    Ok(row_no)
  }

  /// add a quadratic constraint to the model.
  pub fn add_qconstr(&mut self,
                     constrname: &str,
                     lind: &[ffi::c_int],
                     lval: &[ffi::c_double],
                     qrow: &[ffi::c_int],
                     qcol: &[ffi::c_int],
                     qval: &[ffi::c_double],
                     sense: ConstrSense,
                     rhs: ffi::c_double)
                     -> Result<i32> {
    if lind.len() != lval.len() {
      return Err(Error::InconsitentDims);
    }
    if qrow.len() != qcol.len() {
      return Err(Error::InconsitentDims);
    }
    if qcol.len() != qval.len() {
      return Err(Error::InconsitentDims);
    }

    let sense = match sense {
      ConstrSense::Equal => '=' as ffi::c_char,
      ConstrSense::Less => '<' as ffi::c_char,
      ConstrSense::Greater => '>' as ffi::c_char,
    };
    let constrname = try!(make_c_str(constrname));

    let error = unsafe {
      ffi::GRBaddqconstr(self.model,
                         lind.len() as ffi::c_int,
                         lind.as_ptr(),
                         lval.as_ptr(),
                         qrow.len() as ffi::c_int,
                         qrow.as_ptr(),
                         qcol.as_ptr(),
                         qval.as_ptr(),
                         sense,
                         rhs,
                         constrname.as_ptr())
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }

    let qrow_no = self.qconstrs.len() as i32;
    self.qconstrs.push(qrow_no);

    Ok(qrow_no)
  }

  pub fn set_objective(&mut self,
                       lind: &[i32],
                       lval: &[f64],
                       qrow: &[i32],
                       qcol: &[i32],
                       qval: &[f64],
                       sense: ModelSense)
                       -> Result<()> {
    let sense = match sense {
      ModelSense::Minimize => -1,
      ModelSense::Maximize => 1,
    };
    try!(self.set_list(DoubleAttr::Obj, lind, lval));
    try!(self.add_qpterms(qrow, qcol, qval));
    self.set(IntAttr::ModelSense, sense)
  }

  /// add quadratic terms of objective function.
  fn add_qpterms(&mut self,
                 qrow: &[ffi::c_int],
                 qcol: &[ffi::c_int],
                 qval: &[ffi::c_double])
                 -> Result<()> {
    if qrow.len() != qcol.len() {
      return Err(Error::InconsitentDims);
    }
    if qcol.len() != qval.len() {
      return Err(Error::InconsitentDims);
    }

    let error = unsafe {
      ffi::GRBaddqpterms(self.model,
                         qrow.len() as ffi::c_int,
                         qrow.as_ptr(),
                         qcol.as_ptr(),
                         qval.as_ptr())
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }

    Ok(())
  }


  // make an instance of error object related to C API.
  fn error_from_api(&self, errcode: ffi::c_int) -> Error {
    Error::FromAPI(self.env.get_error_msg(), errcode)
  }
}

impl<'a> Drop for Model<'a> {
  fn drop(&mut self) {
    unsafe { ffi::GRBfreemodel(self.model) };
    self.model = null_mut();
  }
}

impl<'a> HasAttr<IntAttr> for Model<'a> {
  type Output = i32;

  fn get(&self, attr: IntAttr) -> Result<i32> {
    let mut value: ffi::c_int = 0;
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error =
      unsafe { ffi::GRBgetintattr(self.model, attrname.as_ptr(), &mut value) };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(value as i32)
  }

  fn set(&mut self, attr: IntAttr, value: i32) -> Result<()> {
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error =
      unsafe { ffi::GRBsetintattr(self.model, attrname.as_ptr(), value) };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(())
  }

  fn get_element(&self, attr: IntAttr, element: i32) -> Result<i32> {
    let mut value: ffi::c_int = 0;
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error = unsafe {
      ffi::GRBgetintattrelement(self.model,
                                attrname.as_ptr(),
                                element,
                                &mut value)
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(value as i32)
  }

  fn set_element(&mut self,
                 attr: IntAttr,
                 element: i32,
                 value: i32)
                 -> Result<()> {
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error = unsafe {
      ffi::GRBsetintattrelement(self.model, attrname.as_ptr(), element, value)
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(())
  }


  fn get_array(&self,
               attr: IntAttr,
               first: usize,
               len: usize)
               -> Result<Vec<i32>> {
    let mut values = Vec::with_capacity(len);
    values.resize(len, 0);
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error = unsafe {
      ffi::GRBgetintattrarray(self.model,
                              attrname.as_ptr(),
                              first as ffi::c_int,
                              len as ffi::c_int,
                              values.as_mut_ptr())
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(values)
  }

  fn set_array(&mut self,
               attr: IntAttr,
               first: usize,
               values: &[i32])
               -> Result<()> {
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error = unsafe {
      ffi::GRBsetintattrarray(self.model,
                              attrname.as_ptr(),
                              first as ffi::c_int,
                              values.len() as ffi::c_int,
                              values.as_ptr())
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(())
  }

  fn get_list(&self, attr: IntAttr, ind: &[i32]) -> Result<Vec<i32>> {
    let mut values = Vec::with_capacity(ind.len());
    values.resize(ind.len(), 0);
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error = unsafe {
      ffi::GRBgetintattrlist(self.model,
                             attrname.as_ptr(),
                             ind.len() as ffi::c_int,
                             ind.as_ptr(),
                             values.as_mut_ptr())
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(values)
  }

  fn set_list(&mut self,
              attr: IntAttr,
              ind: &[i32],
              values: &[i32])
              -> Result<()> {
    if ind.len() != values.len() {
      return Err(Error::InconsitentDims);
    }

    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error = unsafe {
      ffi::GRBsetintattrlist(self.model,
                             attrname.as_ptr(),
                             ind.len() as ffi::c_int,
                             ind.as_ptr(),
                             values.as_ptr())
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(())
  }
}

impl<'a> HasAttr<DoubleAttr> for Model<'a> {
  type Output = f64;

  fn get(&self, attr: DoubleAttr) -> Result<f64> {
    let mut value: ffi::c_double = 0.0;
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error =
      unsafe { ffi::GRBgetdblattr(self.model, attrname.as_ptr(), &mut value) };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(value as f64)
  }

  fn set(&mut self, attr: DoubleAttr, value: f64) -> Result<()> {
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error =
      unsafe { ffi::GRBsetdblattr(self.model, attrname.as_ptr(), value) };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(())
  }

  fn get_element(&self, attr: DoubleAttr, element: i32) -> Result<f64> {
    let mut value: ffi::c_double = 0.0;
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error = unsafe {
      ffi::GRBgetdblattrelement(self.model,
                                attrname.as_ptr(),
                                element,
                                &mut value)
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(value as f64)
  }

  fn set_element(&mut self,
                 attr: DoubleAttr,
                 element: i32,
                 value: f64)
                 -> Result<()> {
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error = unsafe {
      ffi::GRBsetdblattrelement(self.model, attrname.as_ptr(), element, value)
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(())
  }

  fn get_array(&self,
               attr: DoubleAttr,
               first: usize,
               len: usize)
               -> Result<Vec<f64>> {
    let mut values = Vec::with_capacity(len);
    values.resize(len, 0.0);
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error = unsafe {
      ffi::GRBgetdblattrarray(self.model,
                              attrname.as_ptr(),
                              first as ffi::c_int,
                              len as ffi::c_int,
                              values.as_mut_ptr())
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(values)
  }

  fn set_array(&mut self,
               attr: DoubleAttr,
               first: usize,
               values: &[f64])
               -> Result<()> {
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error = unsafe {
      ffi::GRBsetdblattrarray(self.model,
                              attrname.as_ptr(),
                              first as ffi::c_int,
                              values.len() as ffi::c_int,
                              values.as_ptr())
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(())
  }

  fn get_list(&self, attr: DoubleAttr, ind: &[i32]) -> Result<Vec<f64>> {
    let mut values = Vec::with_capacity(ind.len());
    values.resize(ind.len(), 0.0);
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error = unsafe {
      ffi::GRBgetdblattrlist(self.model,
                             attrname.as_ptr(),
                             ind.len() as ffi::c_int,
                             ind.as_ptr(),
                             values.as_mut_ptr())
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(values)
  }

  fn set_list(&mut self,
              attr: DoubleAttr,
              ind: &[i32],
              values: &[f64])
              -> Result<()> {
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error = unsafe {
      ffi::GRBsetdblattrlist(self.model,
                             attrname.as_ptr(),
                             ind.len() as ffi::c_int,
                             ind.as_ptr(),
                             values.as_ptr())
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(())
  }
}

impl<'a> HasAttr<StringAttr> for Model<'a> {
  type Output = String;

  fn get(&self, attr: StringAttr) -> Result<String> {
    let mut value = null();
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error =
      unsafe { ffi::GRBgetstrattr(self.model, attrname.as_ptr(), &mut value) };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(unsafe { from_c_str(value).to_owned() })
  }

  fn set(&mut self, attr: StringAttr, value: String) -> Result<()> {
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let value = try!(make_c_str(value.as_str()));
    let error = unsafe {
      ffi::GRBsetstrattr(self.model, attrname.as_ptr(), value.as_ptr())
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(())
  }

  fn get_element(&self, attr: StringAttr, element: i32) -> Result<String> {
    let mut value = null();
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error = unsafe {
      ffi::GRBgetstrattrelement(self.model,
                                attrname.as_ptr(),
                                element,
                                &mut value)
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(unsafe { from_c_str(value).to_owned() })
  }

  fn set_element(&mut self,
                 attr: StringAttr,
                 element: i32,
                 value: String)
                 -> Result<()> {
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let value = try!(make_c_str(value.as_str()));
    let error = unsafe {
      ffi::GRBsetstrattrelement(self.model,
                                attrname.as_ptr(),
                                element,
                                value.as_ptr())
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(())
  }

  fn get_array(&self,
               attr: StringAttr,
               first: usize,
               len: usize)
               -> Result<Vec<String>> {
    let mut values = Vec::with_capacity(len);
    values.resize(len, null());
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error = unsafe {
      ffi::GRBgetstrattrarray(self.model,
                              attrname.as_ptr(),
                              first as ffi::c_int,
                              len as ffi::c_int,
                              values.as_mut_ptr())
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(values.into_iter()
      .map(|s| unsafe { from_c_str(s).to_owned() })
      .collect())
  }

  fn set_array(&mut self,
               attr: StringAttr,
               first: usize,
               values: &[String])
               -> Result<()> {
    let values = values.into_iter().map(|s| make_c_str(s)).collect::<Vec<_>>();
    if values.iter().any(|ref s| s.is_err()) {
      return Err(Error::StringConversion);
    }
    let values =
      values.into_iter().map(|s| s.unwrap().as_ptr()).collect::<Vec<_>>();

    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error = unsafe {
      ffi::GRBsetstrattrarray(self.model,
                              attrname.as_ptr(),
                              first as ffi::c_int,
                              values.len() as ffi::c_int,
                              values.as_ptr())
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(())
  }

  fn get_list(&self, attr: StringAttr, ind: &[i32]) -> Result<Vec<String>> {
    let mut values = Vec::with_capacity(ind.len());
    values.resize(ind.len(), null());
    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error = unsafe {
      ffi::GRBgetstrattrlist(self.model,
                             attrname.as_ptr(),
                             ind.len() as ffi::c_int,
                             ind.as_ptr(),
                             values.as_mut_ptr())
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(values.into_iter().map(|s| unsafe { from_c_str(s) }).collect())
  }

  fn set_list(&mut self,
              attr: StringAttr,
              ind: &[i32],
              values: &[String])
              -> Result<()> {

    let values = values.into_iter().map(|s| make_c_str(s)).collect::<Vec<_>>();
    if values.iter().any(|ref s| s.is_err()) {
      return Err(Error::StringConversion);
    }
    let values =
      values.into_iter().map(|s| s.unwrap().as_ptr()).collect::<Vec<_>>();

    let attrname = try!(make_c_str(format!("{:?}", attr).as_str()));
    let error = unsafe {
      ffi::GRBsetstrattrlist(self.model,
                             attrname.as_ptr(),
                             ind.len() as ffi::c_int,
                             ind.as_ptr(),
                             values.as_ptr())
    };
    if error != 0 {
      return Err(self.error_from_api(error));
    }
    Ok(())
  }
}


#[test]
fn env_with_logfile() {
  use std::path::Path;
  use std::fs::remove_file;

  let path = Path::new("test_env.log");

  if path.exists() {
    remove_file(path).unwrap();
  }

  {
    let env = Env::new(path.to_str().unwrap()).unwrap();
  }

  assert!(path.exists());
  remove_file(path).unwrap();
}

#[test]
fn param_accesors_should_be_valid() {
  let mut env = Env::new("").unwrap();
  env.set(IntParam::IISMethod, 1).unwrap();
  let iis_method = env.get(IntParam::IISMethod).unwrap();
  assert_eq!(iis_method, 1);
}

// vim: set foldmethod=syntax :
