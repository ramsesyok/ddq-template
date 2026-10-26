//! Job Object（`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`）。Windows だけ。
//!
//! ddq が上げる外部プロセス（PlantUML の JVM、mermaid のヘッドレスブラウザ）を入れておき、
//! ハンドルを閉じたとき（= 使い終わったとき、エラーで抜けたとき、ddq 自身が強制終了されたとき）に
//! **子とその子孫をまとめて終わらせる**。子を kill するだけでは、ブラウザや JVM が起こした
//! 孫プロセスが残り、ddq の標準出力を握ったまま生き続ける（フィルタが `pandoc.pipe` で ddq を
//! 待つので、ビルドごと止まり得る。docs/cli-impl U-0002 の故障注入で実測）。

use std::{
    os::windows::io::AsRawHandle,
    process::Child,
    time::{Duration, Instant},
};

use anyhow::{Result, bail};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0},
    System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_BASIC_PROCESS_ID_LIST, JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectBasicProcessIdList,
        JobObjectExtendedLimitInformation, QueryInformationJobObject, SetInformationJobObject,
        TerminateJobObject,
    },
    System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
};

/// 待つために開いたプロセスのハンドル。Drop で閉じる。
struct Process(HANDLE);

impl Process {
    /// 終わるまで待つ（最大 `limit`）。終わったら true。
    fn wait(&self, limit: Duration) -> bool {
        let ms = limit.as_millis().min(u32::MAX as u128 - 1) as u32;
        unsafe { WaitForSingleObject(self.0, ms) == WAIT_OBJECT_0 }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

/// Job Object のハンドル。Drop で閉じる（中のプロセスはすべて終わる）。
pub struct Handle(HANDLE);

// HANDLE は生ポインタだが、Job Object のハンドルはスレッド間で受け渡してよい
unsafe impl Send for Handle {}

impl Handle {
    /// 閉じる（何度呼んでもよい）。
    pub fn close(&mut self) {
        if !self.0.is_null() {
            unsafe { CloseHandle(self.0) };
            self.0 = std::ptr::null_mut();
        }
    }

    /// 中のプロセスを開く（終了を待つため）。開いた時点で生きているものだけが返る。
    /// ハンドルを持っている限り、PID が再利用されて別のプロセスを待つことはない。
    fn processes(&self) -> Vec<Process> {
        if self.0.is_null() {
            return Vec::new();
        }
        // 可変長の構造体（u32 の件数 2 つのあとに PID（usize）の並び）。ブラウザでも数十程度
        let head = std::mem::offset_of!(JOBOBJECT_BASIC_PROCESS_ID_LIST, ProcessIdList)
            / std::mem::size_of::<usize>();
        let mut buf = vec![0usize; head + 1024];
        let ok = unsafe {
            QueryInformationJobObject(
                self.0,
                JobObjectBasicProcessIdList,
                buf.as_mut_ptr() as *mut _,
                (buf.len() * std::mem::size_of::<usize>()) as u32,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Vec::new();
        }
        let info = unsafe { &*(buf.as_ptr() as *const JOBOBJECT_BASIC_PROCESS_ID_LIST) };
        let listed = (info.NumberOfProcessIdsInList as usize).min(buf.len() - head);
        buf[head..head + listed]
            .iter()
            .filter_map(|&pid| {
                let h = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid as u32) };
                (!h.is_null()).then_some(Process(h))
            })
            .collect()
    }

    /// 中のプロセスをすべて**終わりきるまで**待ってから閉じる。終わりきったら true。
    ///
    /// はじめの `grace` のあいだは自分で終わるのを待ち（ブラウザなら Browser.close のあと
    /// 自分の一時ファイルを片付ける）、過ぎたら止める。全体で最大 `limit`。
    ///
    /// [`close`](Self::close) だけでも中のプロセスは終わるが、終了は非同期で、閉じた直後は
    /// まだファイルを握っていることがある（直後に一時フォルダを消すと失敗する。N-01）。
    /// Job の「動いているプロセス数」が 0 になっても、プロセス自体はまだ終わりきって
    /// いないことがある（実測）ので、プロセスのハンドルを開いてその終了を待つ。
    pub fn end_all(&mut self, grace: Duration, limit: Duration) -> bool {
        let t0 = Instant::now();
        let mut terminated = false;
        let done = loop {
            let procs = self.processes();
            if procs.is_empty() {
                break true;
            }
            for p in &procs {
                loop {
                    let deadline = if terminated { limit } else { grace };
                    let left = deadline.saturating_sub(t0.elapsed());
                    if p.wait(left) {
                        break;
                    }
                    if terminated {
                        break;
                    }
                    unsafe { TerminateJobObject(self.0, 1) };
                    terminated = true;
                }
            }
            if t0.elapsed() >= limit {
                break self.processes().is_empty();
            }
        };
        self.close();
        done
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.close();
    }
}

/// 子プロセスを新しい Job Object に入れる。
pub fn attach(child: &Child) -> Result<Handle> {
    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
            bail!("Job Object を作れません（CreateJobObjectW）");
        }
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            std::mem::size_of_val(&info) as u32,
        ) == 0
        {
            CloseHandle(job);
            bail!("Job Object を設定できません（SetInformationJobObject）");
        }
        if AssignProcessToJobObject(job, child.as_raw_handle() as HANDLE) == 0 {
            CloseHandle(job);
            bail!("子プロセスを Job Object に入れられません（AssignProcessToJobObject）");
        }
        Ok(Handle(job))
    }
}

/// [`attach`] して、入れられなければ子を kill / wait してからエラーを返す
/// （入れられなかった子は、ほかに片付ける持ち主がいない）。
pub fn attach_or_kill(child: &mut Child) -> Result<Handle> {
    attach_or_kill_with(child, attach)
}

fn attach_or_kill_with<H>(child: &mut Child, attach: impl FnOnce(&Child) -> Result<H>) -> Result<H> {
    match attach(child) {
        Ok(h) => Ok(h),
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    fn sleeper() -> Child {
        Command::new("cmd")
            .args(["/c", "ping -n 30 127.0.0.1 >nul"])
            .stdout(Stdio::null())
            .spawn()
            .unwrap()
    }

    #[test]
    fn failed_attach_kills_the_child() {
        // Job Object に入れられなかった場合（故障注入）、子は残らない
        let mut child = sleeper();
        let r: Result<()> = attach_or_kill_with(&mut child, |_| bail!("注入した失敗"));
        assert!(r.is_err());
        assert!(child.try_wait().unwrap().is_some(), "子が生きたまま");
    }

    #[test]
    fn closing_the_job_ends_the_child() {
        let mut child = sleeper();
        let mut job = attach_or_kill(&mut child).unwrap();
        assert!(child.try_wait().unwrap().is_none(), "入れただけで終わった");
        job.close();
        let t0 = std::time::Instant::now();
        while child.try_wait().unwrap().is_none() {
            assert!(t0.elapsed().as_secs() < 5, "Job を閉じても子が終わらない");
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    #[test]
    fn end_all_returns_after_every_process_has_ended() {
        let mut child = sleeper();
        let mut job = attach_or_kill(&mut child).unwrap();
        let t0 = Instant::now();
        assert!(job.end_all(Duration::from_millis(300), Duration::from_secs(5)));
        // 自分では終わらない子なので、猶予（300ms）を待ってから止めている
        assert!(t0.elapsed() >= Duration::from_millis(300), "{:?}", t0.elapsed());
        // 戻った時点で子は終わりきっている（待ち直さない）
        assert!(child.try_wait().unwrap().is_some(), "戻った時点で子が生きている");
    }
}
