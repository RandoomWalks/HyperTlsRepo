use std::collections::{HashMap, Linkedlist};
//! when use Linkedlist vs VecDeque ?
// LL more memory eff. , dont need expand/shrink dynamically
// VecDeque is simpler, no need for dual PTrs 

use std::time::{Duration,  Instant}
//! when use Duration vs Instant ?
// Duration - Elapsed period of time, for comparisons
// Instant - Single point in time 


//! What arc for ? ie  Arc::New(Mutex::New( Hmap::new( entry_id, entry_info ) ))
// can pass ref to map to diffr spawned tasks w/o causing ownership conflicts 


#[derive(Debug)]
struct Connection {
    id: usize, // random uuid or in order  
    last_used: Instant 
    created_at: Instant
}

struct LruConnectionPool {
    max_cap: usize,
    cx_map :HashMap<usize, Connection> // ID-> Cx_Info
    last_used_cx :Linkedlist<LinkedlistNode>, // LRU ordering 
}

struct LinkedlistNode {
    prev:Option<usize>, // ID 
    next:Option<usize>, // ID 
}

impl LruConnectionPool {
    fn new(cap:usize, ) -> Self {
        LruConnectionPool {
            max_cap:cap,
            cx_map: HashMap::new(),
            last_used_cx: Linkedlist:new(),
        } 
    }


    // ! TO DO: REWRITE add_connection() using updated helpers
    fn add_connection(&mut self, cx: Connection  ) {

        // ! CHECK cap 
        if self.max_cap < (cx_map.size() + 1) || self.max_cap < (last_used_cx.size() + 1)   {
            println!("{}", "Error ::- EXCEEDED CAP!");
            return;
        }
        
        //? 1) hmap add 
        self.hmap,insert(cx.id, cx )
        
        //? 1) LL add 

        let new_node = LinkedlistNode {
            prev:None,
            next:None,
        };

            // get LL front 
            if let Some(front_entry) = self.last_used_cx.pop_front() {
                new_node.next = front_entry;
                self.last_used_cx.push_front(front_entry); // LRU : new -> old 
                
            }
        // ! TO FIX : race from readding 2 entries , need to correctly lock for Async/Sync access cases
         self.last_used_cx.push_front(new_node); // LRU : new -> old 
    }
    

    // ! TO DO : Assess API design , compare alternatives  

    fn get_connection(&mut self, conn_id: usize ) -> Option< &Connection> {
        if self.cx_map.contains_key(&conn_id) {
            // update last used entry in hmap 
            update_connection( conn_id);
            let ret_conn = self.cx_map.get(&conn_id);
            return Some(ret_conn)
        } else {
            return None;
        }
    }
        

    fn update_connection(&mut self, conn_id: usize ) {
        // update last used entry in hmap 
        if let Some(pos) = self.last_used_cx.iter().position(|&id| id==conn_id) // ! iter on LL key? where id come from?
        {
            self.last_used_cx.remove(pos);
            return;
        }
        self.last_used_cx.push_back(pos); // LRU: OLD -> NEW 
        
    }
    
    

}



#[tokio::test]
pub fn test_add_connection() {
    // ? 1) create protected pool 
    let pool = Arc:new(Mutex:new( LruConnectionPool:new() ));
    // ? 2) create entry 
    let new_cx = Connection {
        id: 0,
        last_used:Instant::now(),
        created_at:Instant::now(),
    };
    
    pool.add_connection(
        new_cx
    );
    
    // ? 3) insert into pool 
    
}
