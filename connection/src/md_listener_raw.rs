// use libc::c_char;
use libc;
use std::ptr;
use trdp_rs::*;
use std::mem;
use std::ffi::CString;

const RESERVED_MEMORY: u32 = 2000000;
const MD_COMID1: u32 = 1001;

mod trdp_lib;
mod listener;

use crate::trdp_lib::*;


unsafe extern "C" fn md_callback (_a: *mut libc::c_void, app_handle : TRDP_APP_SESSION_T, p_msg: *const TRDP_MD_INFO_T, p_data: *mut u8, data_size :u32)
{

    /*    Check why we have been called    */
    match (*p_msg).resultCode as u32 {
        // Match a single value
        TRDP_NO_ERR => match (*p_msg).msgType as u32 {
            TRDP_MSG_MR => {

                println!("MR Request with reply {}",message_to_string(p_data,data_size));
               
                let response_msg = CString::new("I'm fine, thanx!").unwrap().into_bytes_with_nul();
                let mut src_uri = CString::new("md_listener").unwrap().into_bytes_with_nul();
                let result = tlm_reply(app_handle,
                    &(*p_msg).sessionId,
                    (*p_msg).comId,
                    0,
                    ptr::null_mut(),
                    response_msg.as_ptr() as *const u8,
                    response_msg.len() as u32,
                    src_uri.as_mut_ptr() as *mut i8);
                match result {
                    0 => {}
                    _ => panic!("tlm_reply error"),
                }
            


            }
            _ => panic!("message error {}",(*p_msg).msgType)
        }
        _ => panic!("md_callback error {}",(*p_msg).resultCode)
    }
}


fn main() {


    let own_ip: TRDP_IP_ADDR_T = 0x7f000001; //127.0.0.1
   
    let dest_ip: TRDP_IP_ADDR_T = 0x7f000001; //127.0.0.1

    let dynamic_config = TRDP_MEM_CONFIG_T {
        p: ptr::null_mut(),
        size: RESERVED_MEMORY,
        prealloc: [0; 15],
    };

    let mut host_name = [0 as i8; 17];
    unsafe {
        host_name[0..2].copy_from_slice(std::mem::transmute("Me".as_bytes()));
    }

    let leader_name = [0 as i8; 17];

    let process_config = TRDP_PROCESS_CONFIG_T {

        hostName: host_name,
        leaderName: leader_name,
        cycleTime: 0,
        priority: 0,
        options: TRDP_OPTION_BLOCK as u8,
    };

    let md_configuration = TRDP_MD_CONFIG_T {
        pfCbFunction: Some(md_callback),     //Pointer to MD callback function -> mdCallback
        pRefCon: ptr::null_mut(), // ->  &sSessionData
        sendParam: TRDP_SEND_PARAM_T {
            qos: 0,  //Quality of service (default should be 2 for PD and 2 for MD, TSN priority >= 3)  
            ttl: 64, //Time to live (default should be 64) 
            retries: 0, //D Retries from XML file
            tsn: 0, //if TRUE, do not schedule packet but use TSN socket
            vlan: 0
        },
     
        flags: TRDP_FLAGS_CALLBACK as u8,
     
        replyTimeout: 1000000 as u32,       //Default reply timeout in us
        confirmTimeout: 1000000 as u32,     //Default confirmation timeout in us
        connectTimeout: 1000000 as u32,     //Default connection timeout in us
        sendingTimeout: 1000000 as u32,     //Default sending timeout in us

        udpPort: 17225 as u16,               // Port to be used for UDP MD communication (default: 17225) 
        tcpPort: 17225 as u16,               // Port to be used for TCP MD communication (default: 17225)

        maxNumSessions: 10 as u32,           // Maximal number of replier sessions 
    };


    let com_id = MD_COMID1;
    let mut app_handle: TRDP_APP_SESSION_T = ptr::null_mut();


    unsafe {

        let result = tlc_init(Some(dbg_out), ptr::null_mut(), &dynamic_config as *const _);
        match result {
            0 => {}
            _ => panic!("tlc_init error"),
        }

        let result = tlc_openSession(
            &mut app_handle as *mut _,
            own_ip,              /* own Ip */
            0,                  /* use default IP address    */
            ptr::null_mut(),    /* no Marshalling            */
            ptr::null_mut(),
            &md_configuration as *const _,
            &process_config as *const _,
        );

        match result {
            0 => {}
            _ => panic!("tlc_openSession error"),
        }
    
        let mut listen_tcp: TRDP_LIS_T = ptr::null_mut();


        let result = tlm_addListener(
            app_handle, 
            &mut listen_tcp as *mut _,
            ptr::null_mut(), 
            Some(md_callback),
            1, //true
            com_id,
            0, 0, 
            libc::INADDR_ANY,
            libc::INADDR_ANY,         /*    Source IP filter              */
            dest_ip,
            (TRDP_FLAGS_TCP | TRDP_FLAGS_CALLBACK) as u8, ptr::null_mut(), ptr::null_mut());

        match result {
            0 => {}
            _ => panic!("tlm_addListener error"),
        }

        loop {
            let mut rfds = mem::MaybeUninit::<libc::fd_set>::uninit();

            let mut no_desc: i32 = 0;
            let mut tv = TRDP_TIME_T {
                tv_sec: 0,
                tv_usec: 0,
            };
            let max_tv = TRDP_TIME_T {
                tv_sec: 1,
                tv_usec: 0,
            };
            let min_tv = TRDP_TIME_T {
                tv_sec: 0,
                tv_usec: TRDP_PROCESS_DEFAULT_CYCLE_TIME as i64,
            };

            libc::FD_ZERO(rfds.as_mut_ptr());
            let mut rfds = rfds.assume_init();

            tlc_getInterval(app_handle, &mut tv, &mut rfds, &mut no_desc);

            if vos_cmpTime(&tv, &max_tv) > 0 {
                tv = max_tv;
            }

            if vos_cmpTime(&tv, &min_tv) < 0 {
                tv = min_tv;
            }

            let mut rv = vos_select(
                no_desc + 1,
                &mut rfds,
                ptr::null_mut(),
                ptr::null_mut(),
                &mut tv,
            );

            tlm_process(app_handle, &mut rfds, &mut rv);
           
        }

    }


}