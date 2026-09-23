//! Job Object（`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`）。Windows だけ。
//!
//! ddq が上げる外部プロセス（PlantUML の JVM、mermaid のヘッドレスブラウザ）を入れておき、
//! ハンドルを閉じたとき（= 使い終わったとき、エラーで抜けたとき、ddq 自身が強制終了されたとき）に
//! **子とその子孫をまとめて終わらせる**。子を kill するだけでは、ブラウザや JVM が起こした
//! 孫プロセスが残り、ddq の標準出力を握ったまま生き続ける（フィルタが `pandoc.pipe` で ddq を
//! 待つので、ビルドごと止まり得る。docs/cli-impl U-0002 の故障注入で実測）。

use std::{os::windows::io::AsRawHandle, process::Child};

use anyhow::{Result, bail};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation, SetInformationJobObject,
    },
};

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
}
