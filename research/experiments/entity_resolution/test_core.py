import itertools
import unittest
import core as c


class Checks(unittest.TestCase):
    def test_partition_counts_and_unique_edges(self):
        for n,bell in enumerate([1,2,5,15,52,203,877,4140],1):
            ps=list(c.partitions(n))
            self.assertEqual(len(ps),bell)
            self.assertEqual(len({c.partition_bits(p) for p in ps}),bell)

    def test_all_bitstates_mapping_and_optimum(self):
        for n in range(2,6):
            ws=[(-1)**i*(i+1) for i in range(len(c.edges(n)))]
            model=c.model(n,ws)
            energies=[]
            for xx in itertools.product((0,1),repeat=len(ws)):
                bits=''.join(map(str,xx))
                direct=c.inspect(n,ws,bits)
                binary=sum(w*all(xx[i] for i in vs) for w,vs in model['terms'])
                spin=sum(w*__import__('math').prod(2*xx[i]-1 for i in vs) for w,vs in model['spin_terms'])
                self.assertEqual(binary,8*direct['energy'])
                self.assertEqual(spin,binary)
                energies.append(direct['energy'])
                if direct['violations']:
                    self.assertGreater(direct['energy'],0)
            self.assertEqual(c.inspect(n,ws,c.exact(n,ws))['energy'],min(energies))

    def test_frustrated_triangle(self):
        ws=[2,2,-3]
        self.assertEqual(c.inspect(3,ws,c.threshold(3,ws))['violations'],1)
        self.assertEqual(c.inspect(3,ws,c.closure(3,ws))['score'],1)
        self.assertEqual(c.inspect(3,ws,c.exact(3,ws))['score'],2)
        self.assertEqual(c.inspect(3,ws,c.greedy(3,ws))['violations'],0)

    def test_label_blindness_and_disjoint_coverage(self):
        rows=[{'id':str(i),'title':t,'cluster_id':'poison'} for i,t in enumerate(['a b','a c','a d','x y','x z','q'])]
        a=c.blocks(rows,3)
        for r in rows:r['cluster_id']=object();r['label']=object()
        self.assertEqual(a,c.blocks(list(reversed(rows)),3))
        self.assertEqual(sorted(r['id'] for b in a for r in b),list(map(str,range(6))))
        self.assertTrue(all(set(r)=={'id','title'} for b in a for r in b))

    def test_bad_contracts(self):
        for bits in ['10','112',None]:
            with self.assertRaises(ValueError):c.inspect(3,[1,2,3],bits)
        for weights in [[1],[1,2,float('nan')],[True,2,3]]:
            with self.assertRaises(ValueError):c.exact(3,weights)
        with self.assertRaises(ValueError):c.edges(9)
        with self.assertRaises(ValueError):c.blocks([{'id':'a','title':'x'}]*2)

    def test_metrics_and_zero_ties(self):
        counts=c.confusion(3,'100',['a','a','b'])
        self.assertEqual(c.metrics(counts)['f1'],1)
        self.assertEqual(c.exact(3,[0,0,0]),'000')
        self.assertEqual(c.weights_for([{'title':''},{'title':''}]),[-100])

    def test_reconciliation_boundary(self):
        req={'record_ids':['a','b','c'],'weights':[2,2,-3]}
        result=c.reconcile(req)
        self.assertEqual(result['violations'],0)
        self.assertEqual(result['score'],2)
        self.assertEqual(sorted(i for group in result['clusters'] for i in group),req['record_ids'])
        with self.assertRaises(ValueError):c.reconcile(dict(req,truth=['x','x','y']))
        with self.assertRaises(ValueError):c.reconcile(dict(req,record_ids=['a','a','b']))


if __name__=='__main__':unittest.main()
