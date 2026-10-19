using System;
using System.Collections.Generic;
using System.Reflection;
using System.Runtime.InteropServices;

public static class WordVmlSnapshot {
    static object Get(object target, string name, params object[] arguments) {
        return target.GetType().InvokeMember(name, (name == "Item" ? BindingFlags.InvokeMethod : BindingFlags.GetProperty), null, target, arguments);
    }
    static string Error(Exception exception) {
        while (exception.InnerException != null) exception = exception.InnerException;
        return exception.GetType().Name + ": " + exception.Message;
    }
    static object Read(object target, string name) {
        try {
            object value = Get(target, name);
            return value == null || !Marshal.IsComObject(value) ? value : "COM object";
        } catch (Exception exception) { return new Dictionary<string,object>{{"error",Error(exception)}}; }
    }
    static void Release(object value) {
        if (value != null && Marshal.IsComObject(value)) {
            try { Marshal.FinalReleaseComObject(value); } catch { }
        }
    }
    static Dictionary<string,object> Properties(object target,string member,string[] names) {
        var result=new Dictionary<string,object>();object child=null;
        try {child=Get(target,member);foreach(string name in names)result[name]=Read(child,name);}
        catch(Exception exception) {result["error"]=Error(exception);}
        finally {Release(child);}
        return result;
    }
    static Dictionary<string,object> Position(object range) {
        var result = new Dictionary<string,object>();object duplicate=null;
        try {
            duplicate=Get(range,"Duplicate");
            duplicate.GetType().InvokeMember("Collapse",BindingFlags.InvokeMethod,null,duplicate,new object[]{1});
            result["page"]=Get(duplicate,"Information",3);
            result["x_pt"]=Get(duplicate,"Information",5);
            result["y_pt"]=Get(duplicate,"Information",6);
        } catch(Exception exception) {result["error"]=Error(exception);}
        finally {Release(duplicate);}
        return result;
    }
    public static Dictionary<string,object> Capture(object document) {
        var result = new Dictionary<string,object>();
        var shapes = new List<object>();
        result["shapes"] = shapes;
        foreach (string kind in new []{"Shapes","InlineShapes"}) {
            object collection = null;
            try {
                collection = Get(document, kind);
                int count = Convert.ToInt32(Get(collection,"Count"));
                result[kind+"_count"] = count;
                for (int index = 1; index <= count; ++index) {
                    object shape = null, anchor = null, effect = null, frame = null, text = null, inline = null;
                    var row = new Dictionary<string,object>{{"kind",kind},{"index",index}};
                    shapes.Add(row);
                    try {
                        shape = Get(collection,"Item",index);
                        foreach (string property in new []{"Name","AlternativeText","Title","Type","Width","Height","Left","Top","RelativeHorizontalPosition","RelativeVerticalPosition"}) row[property]=Read(shape,property);
                        row["hyperlink"]=Properties(shape,"Hyperlink",new []{"Address","SubAddress","ScreenTip","TextToDisplay","Type"});
                        row["line"]=Properties(shape,"Line",new []{"Visible","Weight"});
                        row["wrap"]=Properties(shape,"WrapFormat",new []{"Type","Side","DistanceLeft","DistanceRight","DistanceTop","DistanceBottom"});
                        row["shadow"]=Properties(shape,"Shadow",new []{"Visible","Type","OffsetX","OffsetY","Transparency"});
                        object nodes=null;
                        try {
                            nodes=Get(shape,"Nodes");int countNodes=Convert.ToInt32(Get(nodes,"Count"));
                            var nodeValues=new List<object>();row["nodes_count"]=countNodes;row["nodes"]=nodeValues;
                            for(int ni=1;ni<=Math.Min(countNodes,16);ni++) {
                                object node=null;
                                try {
                                    node=Get(nodes,"Item",ni);object points=Get(node,"Points");
                                    var numbers=new List<double>();foreach(object number in (Array)points)numbers.Add(Convert.ToDouble(number));
                                    nodeValues.Add(new Dictionary<string,object>{{"index",ni},{"points",numbers},{"editing",Read(node,"EditingType")},{"segment",Read(node,"SegmentType")}});
                                } catch(Exception exception) {nodeValues.Add(new Dictionary<string,object>{{"index",ni},{"error",Error(exception)}});}
                                finally {Release(node);}
                            }
                        } catch(Exception exception) {row["nodes_error"]=Error(exception);}
                        finally {Release(nodes);}

                        try {
                            anchor = Get(shape,kind=="Shapes" ? "Anchor" : "Range");
                            row["anchor_start"] = Read(anchor,"Start");
                            row["anchor_position"] = Position(anchor);
                            row["page"] = Get(anchor,"Information",3);
                        } catch(Exception exception) { row["anchor_error"]=Error(exception); }
                        try {
                            effect = Get(shape,"TextEffect");
                            var values = new Dictionary<string,object>();
                            foreach(string property in new []{"Text","FontName","FontSize","PresetShape","PresetTextEffect","NormalizedHeight","RotatedChars"}) values[property]=Read(effect,property);
                            row["text_effect"] = values;
                        } catch(Exception exception) { row["text_effect_error"]=Error(exception); }
                        try {
                            frame=Get(shape,"TextFrame"); text=Get(frame,"TextRange"); inline=Get(text,"InlineShapes");
                            var children=new List<object>();
                            int childCount=Convert.ToInt32(Get(inline,"Count"));
                            for(int child=1;child<=childCount;++child) {
                                object item=null;
                                try { item=Get(inline,"Item",child);children.Add(new Dictionary<string,object>{{"index",child},{"type",Read(item,"Type")},{"width",Read(item,"Width")},{"height",Read(item,"Height")}}); }
                                catch(Exception exception) {children.Add(new Dictionary<string,object>{{"index",child},{"error",Error(exception)}});}
                                finally {Release(item);}
                            }
                            row["textbox"] = new Dictionary<string,object>{{"autosize",Read(frame,"AutoSize")},{ "orientation",Read(frame,"Orientation")},{"margin_left",Read(frame,"MarginLeft")},{"margin_right",Read(frame,"MarginRight")},{"margin_top",Read(frame,"MarginTop")},{"margin_bottom",Read(frame,"MarginBottom")},{"font",Properties(text,"Font",new []{"Name","Size"})},{"paragraph",Properties(text,"ParagraphFormat",new []{"SpaceBefore","SpaceAfter","LineSpacing","LineSpacingRule"})},{"text",Read(text,"Text")},{"inline",children}};
                        } catch(Exception exception) {row["textbox_error"]=Error(exception);}
                    } catch(Exception exception) {row["error"]=Error(exception);}
                    finally {Release(inline);Release(text);Release(frame);Release(effect);Release(anchor);Release(shape);}
                }
            } catch(Exception exception) {result[kind+"_error"]=Error(exception);}
            finally {Release(collection);}
        }
        object paragraphs=null;
        var paragraphRows = new List<object>();result["paragraphs"]=paragraphRows;
        try {
            paragraphs=Get(document,"Paragraphs");int count=Convert.ToInt32(Get(paragraphs,"Count"));result["paragraphs_count"]=count;
            for(int index=1;index<=Math.Min(count,32);++index) {
                object paragraph=null, range=null;
                try {paragraph=Get(paragraphs,"Item",index);range=Get(paragraph,"Range");paragraphRows.Add(new Dictionary<string,object>{{"index",index},{"text",Read(range,"Text")},{"start",Read(range,"Start")},{"end",Read(range,"End")},{"page",Get(range,"Information",3)},{"start_position",Position(range)},{ "format",Properties(range,"ParagraphFormat",new []{"SpaceBefore","SpaceAfter","LineSpacing","LineSpacingRule"})},{"font",Properties(range,"Font",new []{"Name","Size"})}});}
                catch(Exception exception) {paragraphRows.Add(new Dictionary<string,object>{{"index",index},{"error",Error(exception)}});}
                finally {Release(range);Release(paragraph);}
            }
        } catch(Exception exception) {result["paragraphs_error"]=Error(exception);}
        finally {Release(paragraphs);}
        result["compatibility_mode"] = Read(document,"CompatibilityMode");
        return result;
    }
}
