using System;
using System.IO;
using System.Text;
using System.Text.Json;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Threading;

// Observation only: exported GDI+ entry points, thread hardware breakpoints.
public sealed class TextGdiObserver {
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool DebugActiveProcess(uint pid);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool DebugActiveProcessStop(uint pid);
 [DllImport("kernel32.dll")] static extern bool DebugSetProcessKillOnExit(bool value);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool WaitForDebugEvent(IntPtr e,uint ms);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool ContinueDebugEvent(uint pid,uint tid,uint status);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool DebugBreakProcess(IntPtr process);
 [DllImport("kernel32.dll",SetLastError=true)] static extern IntPtr OpenProcess(uint access,bool inherit,uint pid);
 [DllImport("kernel32.dll",SetLastError=true)] static extern IntPtr OpenThread(uint access,bool inherit,uint tid);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool GetThreadContext(IntPtr thread,IntPtr context);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool SetThreadContext(IntPtr thread,IntPtr context);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool ReadProcessMemory(IntPtr process,IntPtr address,byte[] data,UIntPtr length,out UIntPtr read);
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern uint GetFinalPathNameByHandleW(IntPtr file,StringBuilder name,uint size,uint flags);
 [DllImport("kernel32.dll",SetLastError=true)] static extern UIntPtr VirtualQueryEx(IntPtr process,IntPtr address,byte[] info,UIntPtr length);
 [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd,out uint pid);
 const uint Handled=0x00010002, Unhandled=0x80010001;
 readonly uint pid; readonly string root; readonly Thread thread;
 readonly Dictionary<uint,IntPtr> threads=new Dictionary<uint,IntPtr>();
 readonly ulong[] addresses=new ulong[3];
 readonly string[] names={"ExtTextOutWImpl","GetTextExtentExPointW","GetCharABCWidthsI"};
 IntPtr process; StreamWriter log; volatile bool stop; public volatile bool Ready;
 public Exception Failure; public int Captures, Saves; bool installed;
 public TextGdiObserver(uint p,string directory) {pid=p;root=directory;Directory.CreateDirectory(root);thread=new Thread(Run);thread.IsBackground=true;}
 public void Start() {thread.Start();for(int i=0;i<300&&!Ready&&Failure==null;i++)Thread.Sleep(100);if(Failure!=null)throw Failure;if(!Ready)throw new Exception("Observer attach exceeded30s");}
 public void Finish() {if(!thread.IsAlive){if(Failure!=null)throw Failure;return;}stop=true;if(process!=IntPtr.Zero&&!DebugBreakProcess(process))throw new Exception("DebugBreakProcess "+Marshal.GetLastWin32Error());if(!thread.Join(15000))throw new Exception("Observer detach exceeded15s");if(Failure!=null)throw Failure;}
 void Log(object value) {log.WriteLine(JsonSerializer.Serialize(value));log.Flush();}
 byte[] Read(ulong address,int size) {var b=new byte[size];UIntPtr count;if(!ReadProcessMemory(process,new IntPtr(unchecked((long)address)),b,(UIntPtr)size,out count)||count.ToUInt64()!=(ulong)size)throw new Exception("ReadProcessMemory "+Marshal.GetLastWin32Error()+" at "+address.ToString("X"));return b;}
 static uint U32(byte[] b,int at)=>BitConverter.ToUInt32(b,at);
 static ulong U64(byte[] b,int at)=>BitConverter.ToUInt64(b,at);
 ulong Pointer(ulong at)=>U64(Read(at,8),0);
 IntPtr Context(IntPtr h,out IntPtr allocation) {allocation=Marshal.AllocHGlobal(1248);var c=new IntPtr((allocation.ToInt64()+15)&~15L);Marshal.Copy(new byte[1232],0,c,1232);Marshal.WriteInt32(c,48,0x10001f);if(!GetThreadContext(h,c)){Marshal.FreeHGlobal(allocation);throw new Exception("GetThreadContext "+Marshal.GetLastWin32Error());}return c;}
 static ulong Reg(IntPtr c,int at)=>(ulong)Marshal.ReadInt64(c,at);
 static void SetReg(IntPtr c,int at,ulong value)=>Marshal.WriteInt64(c,at,(long)value);
 void SaveContext(IntPtr h,IntPtr c) {if(!SetThreadContext(h,c))throw new Exception("SetThreadContext "+Marshal.GetLastWin32Error());}
 void Arm(IntPtr h) {IntPtr a;var c=Context(h,out a);try {for(int i=0;i<3;i++)SetReg(c,72+i*8,addresses[i]);SetReg(c,96,0);SetReg(c,104,0);SetReg(c,112,0x15);SaveContext(h,c);}finally{Marshal.FreeHGlobal(a);}}
 void Clear(IntPtr h) {IntPtr a;var c=Context(h,out a);try {for(int i=0;i<6;i++)SetReg(c,72+i*8,0);SaveContext(h,c);}finally{Marshal.FreeHGlobal(a);}}
 void Module(ulong address,IntPtr file) {
  if(file==IntPtr.Zero||file==new IntPtr(-1))return;
  var s=new StringBuilder(2048);GetFinalPathNameByHandleW(file,s,2048,0);
  Log(new {kind="module",path=s.ToString(),address});
  if(!s.ToString().EndsWith("gdi32full.dll",StringComparison.OrdinalIgnoreCase))return;
  byte[] dos=Read(address,64);byte[] nt=Read(address+U32(dos,60),160);uint er=U32(nt,136);var ed=Read(address+er,40);uint count=U32(ed,24);var nameRvas=Read(address+U32(ed,32),checked((int)count*4));var ordinals=Read(address+U32(ed,36),checked((int)count*2));
  for(int i=0;i<count;i++) {var bytes=Read(address+U32(nameRvas,i*4),160);int end=Array.IndexOf(bytes,(byte)0);string name=Encoding.ASCII.GetString(bytes,0,end<0?bytes.Length:end);int slot=Array.IndexOf(names,name);if(slot>=0){ushort ordinal=BitConverter.ToUInt16(ordinals,i*2);addresses[slot]=address+U32(Read(address+U32(ed,28)+(ulong)ordinal*4,4),0);}}
  if(Array.Exists(addresses,x=>x==0))throw new Exception("Missing GDI full export");
  installed=true;foreach(var h in threads.Values)Arm(h);Log(new {kind="exports",module=s.ToString(),addresses,names});
 }
 bool Hit(uint tid) {
  if(!installed)return false;var h=threads[tid];IntPtr a;var c=Context(h,out a);
  try {
   ulong ip=Reg(c,248),cx=Reg(c,128),dx=Reg(c,136),r8=Reg(c,184),r9=Reg(c,192),sp=Reg(c,152);
   int slot=Array.IndexOf(addresses,ip);if(slot<0)return false;
   Captures++;
   if(Captures<=5000) {
    var head=Read(sp,96);var args=new ulong[]{cx,dx,r8,r9,U64(head,40),U64(head,48),U64(head,56),U64(head,64)};
    string text="";int count=0;string advances="";
    if(slot==0) {
     count=(int)(uint)args[6];
     if(count>4096||count<0)throw new Exception("Unexpected count "+count);
     if(count>0)text=Convert.ToHexString(Read(args[5],count*2));
     if(count>0 && args[7]!=0)advances=Convert.ToHexString(Read(args[7],count*4*((r9&0x2000)!=0?2:1)));
    } else if(slot==1) {
     count=(int)(uint)r8;
     if(count>0 && count<=4096)text=Convert.ToHexString(Read(dx,count*2));
    }
    bool selected=slot==0 && count>1;
    var info=new byte[48];byte[] stack=head,contextBytes=Array.Empty<byte>();
    if(selected) {
     if(VirtualQueryEx(process,new IntPtr((long)sp),info,(UIntPtr)48).ToUInt64()!=48)throw new Exception("VirtualQueryEx failed");
     int stackSize=(int)Math.Min(4096UL,U64(info,0)+U64(info,24)-sp);stack=Read(sp,stackSize);contextBytes=new byte[1232];Marshal.Copy(c,contextBytes,0,1232);
    }
    Log(new {kind="gdi-text",api=names[slot],tid,args,count,textHex=text,advances,sp,stack=Convert.ToHexString(stack),context=Convert.ToHexString(contextBytes),ret=U64(head,0)});
   }
   Marshal.WriteInt32(c,68,Marshal.ReadInt32(c,68)|0x10000);SetReg(c,104,0);SaveContext(h,c);return true;
  } finally {Marshal.FreeHGlobal(a);}
 }
 void Run() {
  IntPtr e=Marshal.AllocHGlobal(176);bool attached=false;uint activeTid=0;bool eventPending=false;
  try {
   log=new StreamWriter(Path.Combine(root,"events.jsonl"),false,new UTF8Encoding(false));process=OpenProcess(0x43A,false,pid);if(process==IntPtr.Zero)throw new Exception("OpenProcess "+Marshal.GetLastWin32Error());
   if(!DebugActiveProcess(pid))throw new Exception("DebugActiveProcess "+Marshal.GetLastWin32Error());attached=true;DebugSetProcessKillOnExit(false);Log(new {kind="attached",pid});
   while(true) {
    if(!WaitForDebugEvent(e,100))continue;
    eventPending=true;uint kind=(uint)Marshal.ReadInt32(e,0),tid=(uint)Marshal.ReadInt32(e,8);activeTid=tid;uint status=Handled;
    if(stop) {foreach(var h in threads.Values)Clear(h);ContinueDebugEvent(pid,tid,Handled);eventPending=false;break;}
    if(kind==2||kind==3) {IntPtr h=OpenThread(0x58,false,tid);if(h==IntPtr.Zero)throw new Exception("OpenThread");threads[tid]=h;if(installed)Arm(h);if(kind==3){IntPtr file=Marshal.ReadIntPtr(e,16);if(file!=IntPtr.Zero)CloseHandle(file);}}
    else if(kind==4) {if(threads.TryGetValue(tid,out var h)){CloseHandle(h);threads.Remove(tid);}}
    else if(kind==5) {Log(new {kind="process-exit",code=Marshal.ReadInt32(e,16)});ContinueDebugEvent(pid,tid,Handled);eventPending=false;attached=false;break;}
    else if(kind==6) {IntPtr file=Marshal.ReadIntPtr(e,16);try {Module((ulong)Marshal.ReadInt64(e,24),file);}finally{if(file!=IntPtr.Zero)CloseHandle(file);}}
    else if(kind==1) {uint code=(uint)Marshal.ReadInt32(e,16);if(code==0x80000004&&Hit(tid))status=Handled;else if(code==0x80000003&&!Ready){Ready=true;Log(new {kind="ready"});}else status=Unhandled;}
    if(!ContinueDebugEvent(pid,tid,status))throw new Exception("ContinueDebugEvent");eventPending=false;
   }
  } catch(Exception ex) {Failure=ex;if(log!=null)Log(new {kind="error",message=ex.ToString()});}
  finally {
   if(eventPending){foreach(var h in threads.Values){try{Clear(h);}catch{}}ContinueDebugEvent(pid,activeTid,Handled);}
   if(attached)DebugActiveProcessStop(pid);foreach(var h in threads.Values)CloseHandle(h);if(process!=IntPtr.Zero){CloseHandle(process);process=IntPtr.Zero;}if(log!=null){Log(new {kind="detached",captures=Captures,saves=Saves});log.Dispose();}Marshal.FreeHGlobal(e);
  }
 }
}
