import init,{partition_request} from './pkg/metis_partition_explorer.js';
const started=performance.now();
try {const wasm=await init();self.onmessage=({data})=>{const start=performance.now();try{const result=partition_request(data.request);self.postMessage({id:data.id,revision:data.revision,result,computationMs:performance.now()-start});}catch(error){self.postMessage({id:data.id,revision:data.revision,error:String(error)});}};self.postMessage({ready:true,initializationMs:performance.now()-started});}catch(error){self.postMessage({startupError:String(error)});}
