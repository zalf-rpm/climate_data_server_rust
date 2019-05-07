// Copyright (c) 2013-2015 Sandstorm Development Group, Inc. and contributors
// Licensed under the MIT License:
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use capnp::primitive_list;
use capnp::Error;

use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};

use capnp::capability::Promise;

use calculator_capnp::calculator;
use climate_data_capnp::climate;

use futures::{Future, Stream};
use tokio::io::AsyncRead;
use tokio::runtime::current_thread;

use chrono::prelude::*;
use std::collections::HashMap;

struct YearlyTavg;

impl YearlyTavg {
    fn new() -> YearlyTavg {
        YearlyTavg
    }

    fn calc_yearly_tavg(
        &self,
        start_date: Date<Utc>,
        end_date: Date<Utc>,
        header: climate::time_series::header_results::Reader,//<'_>,
        data: climate::time_series::data_results::Reader,//<'_>,
    ) -> (Vec<f64>, Vec<f64>) {
        (vec![], vec![])
    }
}

impl climate::identifiable::Server for YearlyTavg {
    fn info(
        &mut self,
        _params: climate::identifiable::InfoParams,
        mut results: climate::identifiable::InfoResults,
    ) -> Promise<(), Error> {
        //results.get().set_info()
        Promise::ok(())
    }
}

fn calc_yearly_tavg(
    start_date: Date<Utc>,
    end_date: Date<Utc>,
    header: climate::time_series::header_results::Reader,//<'_>,
    data: climate::time_series::data_results::Reader,//<'_>,
) -> (Vec<f64>, Vec<f64>) {
    (vec![], vec![])
}

impl climate::model::Server for YearlyTavg {
    fn run_set(
        &mut self,
        params: climate::model::RunSetParams,
        mut result: climate::model::RunSetResults,
    ) -> Promise<(), Error> {
        //::capnp::capability::Promise::err(::capnp::Error::unimplemented("method not implemented".to_string()))
        Promise::ok(())
    }

    fn run(
        &mut self,
        params: climate::model::RunParams,
        mut result: climate::model::RunResults,
    ) -> Promise<(), Error> {
        let ts = pry!(pry!(params.get()).get_time_series());

        //ts.header_request().send();

        Promise::from_future(
            ts.header_request().send().promise.join3(
            ts.data_request().send().promise,
            ts.range_request().send().promise).and_then(move |hdr| {

            let (h, d, r) = hdr;
            let rr = r.get().unwrap();
            let sd = rr.get_start_date().unwrap();
            let ed = rr.get_end_date().unwrap();
            //*
            //let (xs, ys) = self.calc_yearly_tavg(
            let (xs, ys) = calc_yearly_tavg(
                Utc.ymd(
                    sd.get_year().into(),
                    sd.get_month().into(),
                    sd.get_day().into(),
                ),
                Utc.ymd(
                    ed.get_year().into(),
                    ed.get_month().into(),
                    ed.get_day().into(),
                ),
                h.get().unwrap(),
                d.get().unwrap(),
            );
            //*
            let mut xy_result_b = result.get().init_result();
            {
                let mut xsb = xy_result_b.reborrow().init_xs(xs.len() as u32);
                for i in 0..xs.len() {
                    xsb.set(i as u32, xs[i]);
                }
                //xy_result_b.reborrow().set_xs(xsb.into_reader());
            }
            {
                let mut ysb = xy_result_b.reborrow().init_ys(ys.len() as u32);
                for i in 0..xs.len() {
                    ysb.set(i as u32, ys[i]);
                }
                //xy_result_b.reborrow().set_ys(ysb.into_reader());
            }
            //*/

            //result.get().set_result(xy_result_b.into_reader());

            Promise::ok(())
        }))
    }
}

struct ValueImpl {
    value: f64,
}

impl ValueImpl {
    fn new(value: f64) -> ValueImpl {
        ValueImpl { value: value }
    }
}

impl calculator::value::Server for ValueImpl {
    fn read(
        &mut self,
        _params: calculator::value::ReadParams,
        mut results: calculator::value::ReadResults,
    ) -> Promise<(), Error> {
        results.get().set_value(self.value);
        Promise::ok(())
    }
}

fn evaluate_impl(
    expression: calculator::expression::Reader,
    params: Option<primitive_list::Reader<f64>>,
) -> Promise<f64, Error> {
    match pry!(expression.which()) {
        calculator::expression::Literal(v) => Promise::ok(v),
        calculator::expression::PreviousResult(p) => Promise::from_future(
            pry!(p)
                .read_request()
                .send()
                .promise
                .and_then(|v| Ok(v.get()?.get_value())),
        ),
        calculator::expression::Parameter(p) => match params {
            Some(params) if p < params.len() => Promise::ok(params.get(p)),
            _ => Promise::err(Error::failed(format!("bad parameter: {}", p))),
        },
        calculator::expression::Call(call) => {
            let func = pry!(call.get_function());
            let param_promises: Vec<Promise<f64, Error>> = pry!(call.get_params())
                .iter()
                .map(|p| evaluate_impl(p, params))
                .collect();
            // XXX shouldn't need to collect()
            // see https://github.com/alexcrichton/futures-rs/issues/285

            Promise::from_future(::futures::future::join_all(param_promises).and_then(
                move |param_values| {
                    let mut request = func.call_request();
                    {
                        let mut params = request.get().init_params(param_values.len() as u32);
                        for ii in 0..param_values.len() {
                            params.set(ii as u32, param_values[ii]);
                        }
                    }
                    request
                        .send()
                        .promise
                        .and_then(|result| Ok(result.get()?.get_value()))
                },
            ))
        }
    }
}

struct FunctionImpl {
    param_count: u32,
    body: ::capnp_rpc::ImbuedMessageBuilder<::capnp::message::HeapAllocator>,
}

impl FunctionImpl {
    fn new(
        param_count: u32,
        body: calculator::expression::Reader,
    ) -> ::capnp::Result<FunctionImpl> {
        let mut result = FunctionImpl {
            param_count: param_count,
            body: ::capnp_rpc::ImbuedMessageBuilder::new(::capnp::message::HeapAllocator::new()),
        };
        result.body.set_root(body)?;
        Ok(result)
    }
}

impl calculator::function::Server for FunctionImpl {
    fn call(
        &mut self,
        params: calculator::function::CallParams,
        mut results: calculator::function::CallResults,
    ) -> Promise<(), Error> {
        let params = pry!(pry!(params.get()).get_params());
        if params.len() != self.param_count {
            Promise::err(Error::failed(format!(
                "Expect {} parameters but got {}.",
                self.param_count,
                params.len()
            )))
        } else {
            Promise::from_future(
                evaluate_impl(
                    pry!(self.body.get_root::<calculator::expression::Builder>()).into_reader(),
                    Some(params),
                )
                .map(move |v| {
                    results.get().set_value(v);
                }),
            )
        }
    }
}

#[derive(Clone, Copy)]
pub struct OperatorImpl {
    op: calculator::Operator,
}

impl calculator::function::Server for OperatorImpl {
    fn call(
        &mut self,
        params: calculator::function::CallParams,
        mut results: calculator::function::CallResults,
    ) -> Promise<(), Error> {
        let params = pry!(pry!(params.get()).get_params());
        if params.len() != 2 {
            Promise::err(Error::failed("Wrong number of paramters.".to_string()))
        } else {
            let v = match self.op {
                calculator::Operator::Add => params.get(0) + params.get(1),
                calculator::Operator::Subtract => params.get(0) - params.get(1),
                calculator::Operator::Multiply => params.get(0) * params.get(1),
                calculator::Operator::Divide => params.get(0) / params.get(1),
            };
            results.get().set_value(v);
            Promise::ok(())
        }
    }
}

struct CalculatorImpl;

impl calculator::Server for CalculatorImpl {
    fn evaluate(
        &mut self,
        params: calculator::EvaluateParams,
        mut results: calculator::EvaluateResults,
    ) -> Promise<(), Error> {
        Promise::from_future(
            evaluate_impl(pry!(pry!(params.get()).get_expression()), None).map(move |v| {
                results.get().set_value(
                    calculator::value::ToClient::new(ValueImpl::new(v))
                        .into_client::<::capnp_rpc::Server>(),
                );
            }),
        )
    }
    fn def_function(
        &mut self,
        params: calculator::DefFunctionParams,
        mut results: calculator::DefFunctionResults,
    ) -> Promise<(), Error> {
        results.get().set_func(
            calculator::function::ToClient::new(pry!(FunctionImpl::new(
                pry!(params.get()).get_param_count() as u32,
                pry!(pry!(params.get()).get_body())
            )))
            .into_client::<::capnp_rpc::Server>(),
        );
        Promise::ok(())
    }
    fn get_operator(
        &mut self,
        params: calculator::GetOperatorParams,
        mut results: calculator::GetOperatorResults,
    ) -> Promise<(), Error> {
        let op = pry!(pry!(params.get()).get_op());
        results.get().set_func(
            calculator::function::ToClient::new(OperatorImpl { op: op })
                .into_client::<::capnp_rpc::Server>(),
        );
        Promise::ok(())
    }
}

pub fn main() {
    use std::net::ToSocketAddrs;
    let args: Vec<String> = ::std::env::args().collect();
    if args.len() != 3 {
        println!("usage: {} server ADDRESS[:PORT]", args[0]);
        return;
    }

    let addr = args[2]
        .to_socket_addrs()
        .unwrap()
        .next()
        .expect("could not parse address");
    let socket = ::tokio::net::TcpListener::bind(&addr).unwrap();

    //let calc = calculator::ToClient::new(CalculatorImpl).into_client::<::capnp_rpc::Server>();
    let yearly_tavg =
        climate::model::ToClient::new(YearlyTavg).into_client::<::capnp_rpc::Server>();

    let done = socket.incoming().for_each(move |socket| {
        socket.set_nodelay(true)?;
        let (reader, writer) = socket.split();

        let network = twoparty::VatNetwork::new(
            reader,
            std::io::BufWriter::new(writer),
            rpc_twoparty_capnp::Side::Server,
            Default::default(),
        );

        //let rpc_system = RpcSystem::new(Box::new(network), Some(calc.clone().client));
        let rpc_system = RpcSystem::new(Box::new(network), Some(yearly_tavg.clone().client));
        current_thread::spawn(rpc_system.map_err(|e| println!("error: {:?}", e)));
        Ok(())
    });

    current_thread::block_on_all(done).unwrap();
}
