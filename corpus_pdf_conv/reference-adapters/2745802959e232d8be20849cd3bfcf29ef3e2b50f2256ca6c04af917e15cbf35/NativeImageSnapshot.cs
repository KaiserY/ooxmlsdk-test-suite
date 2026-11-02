using System;
using System.Collections.Generic;
using System.Reflection;
using System.Runtime.InteropServices;
public static class NativeImageSnapshot {
 static object Get(object target,string name,params object[] args){return target.GetType().InvokeMember(name,name=="Item"?(BindingFlags.InvokeMethod|BindingFlags.GetProperty):BindingFlags.GetProperty,null,target,args);}
 static void Release(object value){if(value!=null&&Marshal.IsComObject(value))Marshal.ReleaseComObject(value);}
 public static Dictionary<string,object> Capture(object document){object shapes=null;var result=new Dictionary<string,object>();var rows=new List<object>();result["body_shapes"]=rows;result["compatibility_mode"]=Get(document,"CompatibilityMode");result["suppress_before"]=Get(document,"Compatibility",7);result["split_mark"]=Get(document,"Compatibility",60);try{shapes=Get(document,"Shapes");int count=Convert.ToInt32(Get(shapes,"Count"));result["body_shape_count"]=count;for(int i=1;i<=count;i++){object shape=null;try{shape=Get(shapes,"Item",i);rows.Add(new Dictionary<string,object>{{"name",Get(shape,"Name")},{"width",Get(shape,"Width")},{"height",Get(shape,"Height")},{"left",Get(shape,"Left")},{"top",Get(shape,"Top")},{"type",Get(shape,"Type")}});}finally{Release(shape);}}}finally{Release(shapes);}return result;}
}
