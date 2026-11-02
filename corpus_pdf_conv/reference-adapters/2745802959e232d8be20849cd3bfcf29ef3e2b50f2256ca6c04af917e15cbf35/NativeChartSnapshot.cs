using System;
using System.Collections.Generic;
using System.Reflection;
using System.Runtime.InteropServices;

public static class NativeChartSnapshot {
    static object Get(object target, string name, params object[] args) {
        return target.GetType().InvokeMember(name, name == "Item" ? BindingFlags.InvokeMethod : BindingFlags.GetProperty, null, target, args);
    }
    static void Release(object value) {
        if (value != null && Marshal.IsComObject(value)) {
            try { Marshal.FinalReleaseComObject(value); } catch { }
        }
    }
    static object Read(object target, string name) {
        try { return Get(target, name); }
        catch (Exception e) { while (e.InnerException != null) e=e.InnerException; return new Dictionary<string, object>{{"error",e.Message}}; }
    }
    static Dictionary<string, object> Props(object target, params string[] names) {
        var d = new Dictionary<string, object>();
        foreach (string name in names) d[name]=Read(target,name);
        return d;
    }


    public static void ApplyBeforeExport(object document, string mode) {
        if (mode == "none") return;
        if (mode == "repaginate") {
            document.GetType().InvokeMember("Repaginate", BindingFlags.InvokeMethod, null, document, new object[0]);
            return;
        }
        object tocs=null,toc=null;
        try {
            tocs=Get(document,"TablesOfContents");toc=Get(tocs,"Item",1);
            toc.GetType().InvokeMember(mode == "toc" ? "Update" : "UpdatePageNumbers", BindingFlags.InvokeMethod, null, toc, new object[0]);
        } finally { Release(toc);Release(tocs); }
    }

    public static Dictionary<string, object> Capture(object document) {
        var result=new Dictionary<string,object>();var rows=new List<object>();result["entries"]=rows;
        object app=null,sections=null,section=null,footers=null,footer=null,fr=null,fp=null;
        try {
            app=Get(document,"Application");result["application"]=Props(app,"Version","Build","Path");
            string path=Convert.ToString(Get(app,"Path"));var modules=new Dictionary<string,string>();result["module_sha256"]=modules;
            foreach(string name in new[]{"WINWORD.EXE","WWLIB.DLL"}) {
                using(var stream=System.IO.File.OpenRead(System.IO.Path.Combine(path,name)))
                using(var hash=System.Security.Cryptography.SHA256.Create())
                    modules[name]=BitConverter.ToString(hash.ComputeHash(stream)).Replace("-","").ToLowerInvariant();
            }
            sections=Get(document,"Sections");result["section_count"]=Get(sections,"Count");
            for(int n=1;n<=Convert.ToInt32(Get(sections,"Count"));n++) {
                object sec=null,setup=null,cols=null;
                try {sec=Get(sections,"Item",n);setup=Get(sec,"PageSetup");cols=Get(setup,"TextColumns");
                    var row=Props(setup,"SectionStart","PageWidth","PageHeight","TopMargin","LeftMargin","RightMargin");row["section"]=n;row["columns"]=Get(cols,"Count");rows.Add(row);
                }finally{Release(cols);Release(setup);Release(sec);}
            }
            fp=Get(document,"Paragraphs");var parasOut=new List<object>();result["paragraphs"]=parasOut;
            for(int n=1;n<=Convert.ToInt32(Get(fp,"Count"));n++) {
                object para=null,r=null;
                try {para=Get(fp,"Item",n);r=Get(para,"Range");parasOut.Add(new Dictionary<string,object>{{"text",Read(r,"Text")},{"page",Get(r,"Information",3)},{"page_x",Get(r,"Information",5)},{"page_y",Get(r,"Information",6)}});}
                finally {Release(r);Release(para);}
            }
        } finally {Release(fp);Release(fr);Release(footer);Release(footers);Release(section);Release(sections); /* Application is borrowed from the worker RCW. */}

        object tocs=null,toc=null,range=null,paras=null;
        try {
            tocs=Get(document,"TablesOfContents");
            int tocCount=Convert.ToInt32(Get(tocs,"Count"));result["toc_count"]=tocCount;
            if (tocCount==0) {
                object content=null;
                try {content=Get(document,"Content");result["before_update_document_pages"]=Get(content,"Information",4);}
                finally {Release(content);}
                return result;
            }
            toc=Get(tocs,"Item",1);
            object before=null;
            try {before=Get(toc,"Range");string text=Convert.ToString(Get(before,"Text"));result["before_update_text"]=text;
                object bp=null,content=null;
                try {
                    bp=Get(before,"Paragraphs"); result["before_update_paragraphs"]=Read(bp,"Count");
                    content=Get(document,"Content"); result["before_update_document_pages"]=Get(content,"Information",4);
                } finally {Release(content);Release(bp);}
                result["before_update_head"]=text.Substring(0,Math.Min(150,text.Length));}finally{Release(before);}
            toc.GetType().InvokeMember("Update",BindingFlags.InvokeMethod,null,toc,new object[0]);
            result["updated_in_memory_after_original_pdf"]=true;
            range=Get(toc,"Range");paras=Get(range,"Paragraphs");
            for(int i=1;i<=Convert.ToInt32(Get(paras,"Count"));i++) {
                object p=null,r=null,f=null,ts=null;
                try {
                    p=Get(paras,"Item",i);r=Get(p,"Range");f=Get(p,"Format");ts=Get(f,"TabStops");
                    var row=Props(f,"LeftIndent","FirstLineIndent","RightIndent");row["ordinal"]=i;row["text"]=Read(r,"Text");
                    if (i == 1) row["word_open_xml"]=Read(r,"WordOpenXML");
                    rows.Add(row);var tabs=new List<object>();row["tabs"]=tabs;
                    for(int j=1;j<=Convert.ToInt32(Get(ts,"Count"));j++){object t=null;try{t=Get(ts,"Item",j);tabs.Add(Props(t,"Position","Alignment","Leader"));}finally{Release(t);}}
                    
                }finally{Release(ts);Release(f);Release(r);Release(p);}
            }
        }finally{Release(paras);Release(range);Release(toc);Release(tocs);}
        return result;
    }
}
