import React,{useState,useEffect} from "react";
import {createRoot} from "react-dom/client";
import {FluentProvider,webLightTheme,webDarkTheme,Switch,Button,Card,CardHeader,Text,Caption1,Divider,Dropdown,Option,Slider,Field,Input,Link,TabList,Tab,tokens} from "@fluentui/react-components";
import {Settings20Regular,CalendarMonth20Regular,Info20Regular,DocumentBulletList20Regular,Clock20Regular,ClockDismiss20Regular,CalendarCancel20Regular,ArrowSync20Regular,ArrowSyncRegular,ClipboardRegular,ClipboardCheckmarkRegular,Delete20Regular,Edit20Regular,Play20Regular,Stop20Regular} from "@fluentui/react-icons";
import {DatePicker,defaultDatePickerStrings} from "@fluentui/react-datepicker-compat";
import uk from "../../src/locales/uk.json";
import clRaw from "../../CHANGELOG.yaml?raw";
import {parse} from "yaml";
import logo from "../../public/logo.png";
const D={uk,ch:uk.changelog,cl:parse(clRaw).slice(0,4)};
const T=p=>p.split(".").reduce((o,k)=>o?.[k],D.uk)??p;
const Sec=({k})=><Text size={100} weight="semibold" block style={{color:tokens.colorNeutralForeground3}}>{T("sections."+k)}</Text>;
const Lbl=({k})=><div style={{flex:1,minWidth:0}}><Text weight="medium" block>{T(`controls.${k}.label`)}</Text><Caption1 style={{color:tokens.colorNeutralForeground3}}>{T(`controls.${k}.description`)}</Caption1></div>;
const SwRow=({k,v,set,off})=><div style={{display:"flex",alignItems:"center",gap:12,opacity:off?.5:1}}><Lbl k={k}/><Switch checked={v} disabled={off} onChange={(_,d)=>set(d.checked)}/></div>;
const DdRow=({k,v,set,opts})=><div style={{display:"flex",flexDirection:"column",gap:8}}><Lbl k={k}/><Dropdown style={{minWidth:0,width:"100%"}} value={opts.find(o=>o[0]===v)?.[1]} selectedOptions={[v]} onOptionSelect={(_,d)=>set(d.optionValue)}>{opts.map(([a,b])=><Option key={a} value={a}>{b}</Option>)}</Dropdown></div>;
const SlRow=({k,v,set,min,max,unit})=><div style={{display:"flex",flexDirection:"column",gap:4}}><Lbl k={k}/><div style={{display:"flex",alignItems:"center",gap:12}}><Slider style={{flex:1}} min={min} max={max} value={v} onChange={(_,d)=>set(d.value)}/><Text style={{minWidth:48,textAlign:"right"}}>{v}{unit}</Text></div></div>;
const cardSt={display:"flex",flexDirection:"column",gap:12};
const pad=n=>String(n).padStart(2,"0");
function Overlay({close}){const[t,setT]=useState(60);useEffect(()=>{const i=setInterval(()=>setT(x=>x-1),1000);return()=>clearInterval(i)},[]);useEffect(()=>{t<=0&&close()},[t]);
return <div style={{position:"absolute",inset:0,zIndex:5,background:"#000",color:"#fff",display:"flex",flexDirection:"column",alignItems:"center",justifyContent:"center",gap:16,padding:32,textAlign:"center"}}>
<div style={{fontSize:26,fontWeight:300,letterSpacing:4}}>{T("overlay.title")}</div><div style={{color:"#aaa",letterSpacing:3,fontSize:13}}>{T("overlay.subtitle")}</div>
<div style={{fontSize:56,fontWeight:200,fontVariantNumeric:"tabular-nums",margin:"24px 0"}}>{pad(Math.floor(t/60))}:{pad(t%60)}</div>
<Button appearance="outline" icon={<Stop20Regular/>} onClick={close} style={{color:"#fff"}}>{T("buttons.stop")}</Button></div>}
function SettingsTab({s,set,status,setStatus}){const[syncing,setSy]=useState(false),[cp,setCp]=useState({});
const flash=(k,m)=>{setCp(c=>({...c,[k]:m}));setTimeout(()=>setCp(c=>({...c,[k]:null})),1500)};
const sc=T("controls.schedule_card"),off=!s.ceremony;
return <>
<Card><CardHeader image={<div style={{display:"grid",placeItems:"center",width:40,height:40,borderRadius:8,background:tokens.colorBrandBackground2,color:tokens.colorBrandForeground1}}>{off?<ClockDismiss20Regular/>:status.skip?<CalendarCancel20Regular/>:<Clock20Regular/>}</div>}
header={<Text weight="semibold">{off?sc.disabled_title:status.skip?sc.skipped_title:sc.scheduled_title}</Text>}
description={<Caption1>{off?sc.disabled_desc:status.skip?sc.skipped_desc:s.weekdays?sc.scheduled_desc_weekdays:sc.scheduled_desc_daily}</Caption1>}
action={!off?<Button appearance={status.skip?"primary":"secondary"} onClick={()=>setStatus({...status,skip:!status.skip})}>{status.skip?sc.resume_button:sc.skip_button}</Button>:undefined}/></Card>
<Card style={cardSt}><Sec k="general"/>
<SwRow k="ceremony" v={s.ceremony} set={v=>set("ceremony",v)}/><Divider/>
<SwRow k="autostart" v={s.autostart} set={v=>set("autostart",v)}/><Divider/>
<SwRow k="weekdays" v={s.weekdays} set={v=>set("weekdays",v)}/><Divider/>
<SwRow k="use_system_theme" v={s.sysTheme} set={v=>set("sysTheme",v)}/>
{!s.sysTheme&&<DdRow k="ui_theme" v={s.theme} set={v=>set("theme",v)} opts={[["light",T("controls.ui_theme.light")],["dark",T("controls.ui_theme.dark")]]}/>}<Divider/>
<SlRow k="grace" v={s.grace} set={v=>set("grace",v)} min={0} max={5} unit={" "+T("controls.grace.unit")}/><Divider/>
<SwRow k="system_time" v={s.sysTime} set={v=>set("sysTime",v)}/><Divider/>
<SwRow k="show_nearby_personal_dates" v={s.nearby} set={v=>set("nearby",v)}/></Card>
<Card style={cardSt}><Sec k="reminder"/><DdRow k="reminder" v={s.reminder} set={v=>set("reminder",v)} opts={[["0",T("controls.reminder.immediately")],...[1,5,10].map(n=>[String(n),`${T("controls.reminder.in")} ${n} ${T("controls.grace.unit")}`])]}/></Card>
<Card style={cardSt}><Sec k="audio_mode"/>
<SwRow k="overlay" v={s.overlay} set={v=>set("overlay",v)}/><Divider/>
<SwRow k="flag_animation" v={s.flag} off={!s.overlay} set={v=>set("flag",v)}/><Divider/>
<DdRow k="voice" v={s.voice} set={v=>set("voice",v)} opts={["bohdan_hdal","sonia_sotnyk","dania_khomutovskyi","radio_bg","vinnytsia_city"].map(k=>[k,T("controls.voice."+k)])}/><Divider/>
<Caption1 style={{color:tokens.colorNeutralForeground3}}>{T("controls.constructor.description")}</Caption1>
<SwRow k="constructor.opening_bell" v={s.bell} set={v=>set("bell",v)}/>
<SwRow k="constructor.announcement" v={s.ann} set={v=>set("ann",v)}/>
<DdRow k="constructor.body" v={s.body} set={v=>set("body",v)} opts={[["metronome",T("controls.constructor.body.metronome")],["silence",T("controls.constructor.body.silence")]]}/>
<DdRow k="constructor.closing" v={s.closing} set={v=>set("closing",v)} opts={["none","ending","bell","anthem"].map(k=>[k,T("controls.constructor.closing."+k)])}/>
{s.closing==="anthem"&&<DdRow k="anthem_voice" v={s.anthem} set={v=>set("anthem",v)} opts={["default","mykhailo_khoma","oleksandr_ponomarov"].map(k=>[k,T("controls.anthem_voice."+k)])}/>}<Divider/>
<SlRow k="volume" v={s.volume} set={v=>set("volume",v)} min={0} max={100} unit="%"/></Card>
<Card style={cardSt}><Sec k="system"/>
<SwRow k="pause" v={s.pause} set={v=>set("pause",v)}/><Divider/>
<SwRow k="resume" v={s.resume} off={!s.pause} set={v=>set("resume",v)}/><Divider/>
<SwRow k="volume_priority" v={s.prio} set={v=>set("prio",v)}/><Divider/>
<SwRow k="auto_unmute" v={s.unmute} set={v=>set("unmute",v)}/></Card>
<Card style={cardSt}><Sec k="data"/>
{["export","import"].map((k,i)=><React.Fragment key={k}>{i>0&&<Divider/>}<div style={{display:"flex",flexDirection:"column",gap:8}}><Lbl k={k}/><Button appearance="secondary" icon={<ClipboardRegular/>} onClick={()=>flash(k,T(`controls.${k}.${k==="export"?"copied":"success"}`))}>{cp[k]||T(`controls.${k}.button`)}</Button></div></React.Fragment>)}</Card>
<Card><div style={{display:"flex",flexDirection:"column",gap:4,fontSize:14}}><div>{T("status.last_ceremony")}: <strong>{status.last}</strong></div>
<div style={{display:"flex",alignItems:"center",justifyContent:"space-between"}}><span>{T("status.ntp_sync")}: <strong>{status.ntp}</strong></span>
<Button size="small" appearance="transparent" disabled={syncing} icon={<ArrowSync20Regular/>} onClick={()=>{setSy(true);setTimeout(()=>{setSy(false);setStatus({...status,ntp:new Date().toLocaleString("uk-UA")})},900)}}>{T("buttons.sync")}</Button></div></div></Card></>}
function Dates(){const[list,setList]=useState([]),[date,setDate]=useState(null),[name,setName]=useState("");const P=T("personal_dates");
const fmt=d=>d?d.toLocaleDateString("uk-UA"):"";
return <><Card style={cardSt}><Text weight="semibold">{P.title}</Text>
<Field label={P.date}><DatePicker value={date} onSelectDate={setDate} formatDate={fmt} strings={{...defaultDatePickerStrings,...D.uk.calendar}}/></Field>
<Field label={P.label}><Input value={name} onChange={(_,d)=>setName(d.value)}/></Field>
<Button appearance="primary" disabled={!date||!name} onClick={()=>{setList([...list,{date,name,id:Date.now()}]);setDate(null);setName("")}}>{T("buttons.add")}</Button></Card>
<Card style={cardSt}><Text weight="semibold">{P.list_title}</Text>{list.length===0?<Caption1>{P.empty}</Caption1>:list.map(x=><div key={x.id} style={{display:"flex",alignItems:"center",gap:8}}><div style={{flex:1}}><Text weight="medium" block>{x.name}</Text><Caption1>{fmt(x.date)}</Caption1></div><Button appearance="subtle" icon={<Edit20Regular/>} aria-label={T("buttons.edit")}/><Button appearance="subtle" icon={<Delete20Regular/>} aria-label={T("buttons.delete")} onClick={()=>setList(list.filter(y=>y.id!==x.id))}/></div>)}</Card></>}
function About(){const A=T("about"),[m,setM]=useState({});const f=(k,v)=>{setM({...m,[k]:v});setTimeout(()=>setM({}),1500)};
return <div style={{display:"flex",flexDirection:"column",alignItems:"center",textAlign:"center",padding:"24px 8px",gap:16}}>
<img src={logo} alt="" style={{width:96,height:96,borderRadius:8}} />
<div style={{display:"flex",flexDirection:"column",gap:6}}><Text size={500} weight="semibold">{T("app.title")}</Text><Caption1 style={{color:tokens.colorNeutralForeground3}}>{A.version} 0.12.1</Caption1></div>
<div style={{display:"flex",flexDirection:"column",gap:10,color:tokens.colorNeutralForeground2}}><Text>{A.description}</Text><Text>{A.acknowledgments}</Text></div>
<div style={{display:"flex",flexDirection:"column",gap:8}}><Link href="https://bohdan.com.ua/memoryminute" target="_blank">bohdan.com.ua/memoryminute</Link><Link href="https://github.com/ChernegaSergiy/minute-of-silence" target="_blank">github.com/ChernegaSergiy/minute-of-silence</Link></div>
<div style={{display:"flex",justifyContent:"center",gap:8,flexWrap:"wrap"}}><Button appearance="subtle" icon={<ArrowSyncRegular/>} onClick={()=>{f("u",A.check_updates_checking);setTimeout(()=>f("u",A.check_updates_up_to_date),900)}}>{m.u||A.check_updates}</Button><Button appearance="subtle" icon={m.l===A.copy_logs_copied?<ClipboardCheckmarkRegular/>:<ClipboardRegular/>} onClick={()=>f("l",A.copy_logs_copied)}>{m.l||A.copy_logs}</Button></div>
<div style={{marginTop:8,color:tokens.colorNeutralForeground3,fontSize:12}}>{A.license}<br/>{A.glory}</div>
</div>}
function Changelog(){return <>{D.cl.map(v=><Card key={v.version} style={cardSt}><Text size={200} weight="semibold">v{v.version} — {v.date}</Text>{[...new Set(v.entries.map(e=>e.category))].map(c=><div key={c} style={{marginTop:8}}><Text size={100} weight="semibold" block style={{textTransform:"uppercase",marginBottom:4}}>{D.ch.categories[c]}</Text><ul style={{margin:0,paddingLeft:20}}>{v.entries.filter(e=>e.category===c).map(e=><li key={e.key} style={{fontSize:13,lineHeight:"18px",marginBottom:2}}>{D.ch[v.version]?.[e.key]??e.key}</li>)}</ul></div>)}</Card>)}</>}
const S0={ceremony:true,autostart:true,weekdays:false,sysTheme:true,theme:"dark",grace:2,sysTime:false,nearby:true,reminder:"5",overlay:true,flag:true,voice:"bohdan_hdal",bell:true,ann:true,body:"metronome",closing:"anthem",anthem:"default",volume:70,pause:true,resume:true,prio:false,unmute:true};
function App(){const[tab,setTab]=useState("settings"),[s,setS]=useState(S0),[status,setStatus]=useState({skip:false,last:"—",ntp:"—"}),[saved,setSaved]=useState(S0),[ov,setOv]=useState(false);
const mq=window.matchMedia("(prefers-color-scheme: dark)"),[sys,setSys]=useState(mq.matches);useEffect(()=>{const f=e=>setSys(e.matches);mq.addEventListener("change",f);return()=>mq.removeEventListener("change",f)},[]);
const dirty=JSON.stringify(s)!==JSON.stringify(saved),dark=s.sysTheme?sys:s.theme==="dark";
const nav=[["settings",<Settings20Regular/>],["personal_dates",<CalendarMonth20Regular/>],["about",<Info20Regular/>],["changelog",<DocumentBulletList20Regular/>]];
return <FluentProvider theme={dark?webDarkTheme:webLightTheme} style={{height:"100%",background:"transparent"}}><div className="phone" style={{background:tokens.colorNeutralBackground2,color:tokens.colorNeutralForeground1}}>
<header style={{background:tokens.colorNeutralBackground1,padding:"14px 16px",borderBottom:`1px solid ${tokens.colorNeutralStroke2}`}}><Text size={500} weight="semibold">{T("app.title")}</Text></header>
<main style={{display:"flex",flexDirection:"column",gap:12,flex:1,overflowY:"auto",padding:12}}>{tab==="settings"&&<SettingsTab {...{s,status,setStatus}} set={(k,v)=>setS({...s,[k]:v})}/>}{tab==="personal_dates"&&<Dates/>}{tab==="about"&&<About/>}{tab==="changelog"&&<Changelog/>}</main>
{tab==="settings"&&<div style={{display:"flex",gap:8,padding:"8px 12px",background:tokens.colorNeutralBackground1,borderTop:`1px solid ${tokens.colorNeutralStroke2}`}}><Button style={{flex:1}} icon={<Play20Regular/>} onClick={()=>setOv(true)}>{T("buttons.test")}</Button><Button style={{flex:1}} appearance="primary" disabled={!dirty} onClick={()=>setSaved(s)}>{dirty?T("buttons.save_dirty"):T("buttons.save")}</Button></div>}
<nav style={{background:tokens.colorNeutralBackground1,borderTop:`1px solid ${tokens.colorNeutralStroke2}`,paddingBottom:"env(safe-area-inset-bottom,0px)"}}><TabList selectedValue={tab} onTabSelect={(_,d)=>setTab(d.value)} style={{display:"flex"}}>{nav.map(([k,i])=><Tab key={k} value={k} style={{flex:1,minWidth:0,flexDirection:"column",height:56,padding:"4px 0"}}>{i}<span style={{fontSize:9,letterSpacing:0,whiteSpace:"nowrap"}}>{T("tabs."+k)}</span></Tab>)}</TabList></nav>
{ov&&<Overlay close={()=>setOv(false)}/>}</div></FluentProvider>}
createRoot(document.getElementById("root")).render(<App/>);
