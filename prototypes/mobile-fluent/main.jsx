import React,{useState,useEffect} from "react";
import {createRoot} from "react-dom/client";
import {FluentProvider,webLightTheme,webDarkTheme,Switch,Button,Card,CardHeader,Text,Caption1,Divider,Dropdown,Option,Slider,Field,Input,TabList,Tab,tokens} from "@fluentui/react-components";
import {Settings20Regular,CalendarMonth20Regular,Info20Regular,DocumentBulletList20Regular,Clock20Regular,ClockDismiss20Regular,CalendarCancel20Regular,ArrowSync20Regular,ClipboardRegular,Delete20Regular,Edit20Regular,Play20Regular,Stop20Regular} from "@fluentui/react-icons";
import uk from "../../src/locales/uk.json";
const D={uk};
const T=p=>p.split(".").reduce((o,k)=>o?.[k],D.uk)??p;
function App(){const[tab,setTab]=useState("settings");
const mq=window.matchMedia("(prefers-color-scheme: dark)"),[sys,setSys]=useState(mq.matches);useEffect(()=>{const f=e=>setSys(e.matches);mq.addEventListener("change",f);return()=>mq.removeEventListener("change",f)},[]);
const dark=sys;
const nav=[["settings",<Settings20Regular/>],["personal_dates",<CalendarMonth20Regular/>],["about",<Info20Regular/>],["changelog",<DocumentBulletList20Regular/>]];
return <FluentProvider theme={dark?webDarkTheme:webLightTheme} style={{height:"100%",background:"transparent"}}><div className="phone" style={{background:tokens.colorNeutralBackground2,color:tokens.colorNeutralForeground1}}>
<header style={{background:tokens.colorNeutralBackground1,padding:"14px 16px",borderBottom:`1px solid ${tokens.colorNeutralStroke2}`}}><Text size={500} weight="semibold">{T("app.title")}</Text></header>
<main style={{display:"flex",flexDirection:"column",gap:12,flex:1,overflowY:"auto",padding:12}}></main>
<nav style={{background:tokens.colorNeutralBackground1,borderTop:`1px solid ${tokens.colorNeutralStroke2}`,paddingBottom:"env(safe-area-inset-bottom,0px)"}}><TabList selectedValue={tab} onTabSelect={(_,d)=>setTab(d.value)} style={{display:"flex"}}>{nav.map(([k,i])=><Tab key={k} value={k} style={{flex:1,minWidth:0,flexDirection:"column",height:56,padding:"4px 0"}}>{i}<span style={{fontSize:9,letterSpacing:0,whiteSpace:"nowrap"}}>{T("tabs."+k)}</span></Tab>)}</TabList></nav>
</div></FluentProvider>}
createRoot(document.getElementById("root")).render(<App/>);
