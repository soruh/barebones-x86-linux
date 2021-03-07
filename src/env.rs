pub struct Environment {
    args_start: *const *const u8,
    env_start: *const *const u8,
}

impl Environment {
    fn n_args(&self) -> usize {
        (unsafe { self.env_start.offset_from(self.args_start) - 1 }) as usize
    }

    pub unsafe fn from_raw_parts(n_args: usize, args_start: *const *const u8) -> Self {
        let env_start = args_start.add(n_args + 1);

        let res = Self {
            args_start,
            env_start,
        };

        assert_eq!(res.n_args(), n_args);

        res
    }

    pub fn args(&self) -> Args {
        Args {
            i: 0,
            n: self.n_args(),
            arg_ptr: self.args_start,
        }
    }

    pub fn env(&self) -> Env {
        Env {
            env_ptr: self.env_start,
        }
    }

    pub fn arg(&self, i: usize) -> Option<&'static str> {
        if i < self.n_args() {
            let string = unsafe { read_str(*self.args_start.add(i)) };

            Some(string)
        } else {
            None
        }
    }
}

unsafe fn read_str(ptr: *const u8) -> &'static str {
    let n = {
        let mut p = ptr;
        while *p != 0 {
            p = p.add(1);
        }

        p as usize - ptr as usize
    };

    let bytes = core::slice::from_raw_parts(ptr, n);

    core::str::from_utf8(bytes).expect("enviroment had non UTF-8 string")
}

pub struct Args {
    i: usize,
    n: usize,
    arg_ptr: *const *const u8,
}

impl Iterator for Args {
    type Item = &'static str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.n {
            self.i += 1;
            Some(unsafe { read_str(*self.arg_ptr.add(self.i - 1)) })
        } else {
            None
        }
    }
}

pub struct Env {
    env_ptr: *const *const u8,
}

impl Iterator for Env {
    type Item = (&'static str, &'static str);

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let ptr = *self.env_ptr;

            if ptr.is_null() {
                return None;
            }

            self.env_ptr = self.env_ptr.add(1);

            let res = read_str(ptr);

            let equals_index = res
                .bytes()
                .enumerate()
                .find_map(|(i, x)| (x == b'=').then(|| i))
                .unwrap_or_else(|| res.len());

            let (key, value) = res.split_at(equals_index);

            Some((key, &value[1..]))
        }
    }
}
