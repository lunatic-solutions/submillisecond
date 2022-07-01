// use lunatic::net::TcpStream;
// use lunatic_test::test;
// use std::io::Write;

// const RAW_REQUEST: &[u8] = r"
// GET /api/mgmt/alive HTTP/1.1
// Host: localhost:3000

// GET /api/mgmt/alive HTTP/1.1
// Host: localhost:3000

// "
// .as_bytes();

// #[test]
// fn pipelining_basic() {
//     let mut stream = TcpStream::connect("localhost:3000").unwrap();

//     match stream.write_all(RAW_REQUEST) {
//         Ok(_) => assert_eq!(RAW_REQUEST.len(), 10),
//         Err(e) => {
//             panic!("Failed to write to stream {:?}", e)
//         }
//     }
// }
